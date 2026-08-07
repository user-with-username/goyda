use jni::{
    objects::{JObject, JValue, GlobalRef},
    JNIEnv, JavaVM
};
use crate::core::{Backend, BackendUpdater};
use crate::components::{LayoutDirection, StyleProperty, Color, Axis, Edge, StyleValue};
use crate::components::style::SPACING;
use crate::core::events::Update;
use std::collections::HashMap;
use std::sync::Arc;
use once_cell::sync::{Lazy, OnceCell};

pub static JVM: OnceCell<JavaVM> = OnceCell::new();

type StyleApplier = fn(&mut JNIEnv, &JObject, &StyleValue);

fn map_color(color: Color) -> i32 {
    match color {
        Color::PRIMARY => 0xFF6200EEu32 as i32,
        Color::GRAY => 0xFF888888u32 as i32,
        Color::GREEN => 0xFF4CAF50u32 as i32,
        Color::RED => 0xFFF44336u32 as i32,
        Color::BACKGROUND => 0xFFFFFFFFu32 as i32,
        Color::Custom(hex) => hex as i32,
    }
}

fn resolve_color(value: &StyleValue) -> Option<i32> {
    match value {
        StyleValue::Color(c) => Some(map_color(*c)),
        _ => None,
    }
}

fn resolve_length(value: &StyleValue) -> Option<i32> {
    match value {
        StyleValue::Length(v) => Some((*v * 3.0) as i32),
        StyleValue::Spacing(scale) => {
            let v = SPACING.get(*scale).copied();
            if v.is_none() {
                #[cfg(debug_assertions)]
                eprintln!("goyda(android): spacing scale index {scale} out of range");
            }
            v.map(|v| (v * 3.0) as i32)
        }
        _ => None,
    }
}

fn resolve_font_size(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Length(v) => Some(*v),
        _ => None,
    }
}

fn get_padding(env: &mut JNIEnv, view: &JObject) -> (i32, i32, i32, i32) {
    let l = env.call_method(view, "getPaddingLeft", "()I", &[]).unwrap().i().unwrap();
    let t = env.call_method(view, "getPaddingTop", "()I", &[]).unwrap().i().unwrap();
    let r = env.call_method(view, "getPaddingRight", "()I", &[]).unwrap().i().unwrap();
    let b = env.call_method(view, "getPaddingBottom", "()I", &[]).unwrap().i().unwrap();
    (l, t, r, b)
}

fn set_padding(env: &mut JNIEnv, view: &JObject, l: i32, t: i32, r: i32, b: i32) {
    env.call_method(
        view, "setPadding", "(IIII)V",
        &[JValue::Int(l), JValue::Int(t), JValue::Int(r), JValue::Int(b)],
    ).unwrap();
}

fn apply_text_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else { return; };
    env.call_method(view, "setTextColor", "(I)V", &[JValue::Int(c)]).unwrap();
}

fn get_or_create_gradient_drawable<'a>(env: &mut JNIEnv<'a>, view: &JObject) -> JObject<'a> {
    let existing = env
        .call_method(view, "getBackground", "()Landroid/graphics/drawable/Drawable;", &[])
        .ok()
        .and_then(|r| r.l().ok());

    let is_gradient = existing
        .as_ref()
        .map(|d| !d.is_null() && env.is_instance_of(d, "android/graphics/drawable/GradientDrawable").unwrap_or(false))
        .unwrap_or(false);

    if is_gradient {
        existing.unwrap()
    } else {
        let (l, t, r, b) = get_padding(env, view);

        let drawable = env.new_object("android/graphics/drawable/GradientDrawable", "()V", &[]).unwrap();
        env.call_method(
            view, "setBackground", "(Landroid/graphics/drawable/Drawable;)V",
            &[JValue::Object(&drawable)],
        ).unwrap();

        let is_clickable = env.call_method(view, "isClickable", "()Z", &[]).unwrap().z().unwrap();
        if is_clickable && l == 0 && t == 0 {
            set_padding(env, view, 48, 32, 48, 32);
        } else {
            set_padding(env, view, l, t, r, b);
        }

        drawable
    }
}

fn get_or_create_foreground_drawable<'a>(env: &mut JNIEnv<'a>, view: &JObject) -> JObject<'a> {
    let existing = env
        .call_method(view, "getForeground", "()Landroid/graphics/drawable/Drawable;", &[])
        .ok()
        .and_then(|r| r.l().ok());

    let is_gradient = existing
        .as_ref()
        .map(|d| !d.is_null() && env.is_instance_of(d, "android/graphics/drawable/GradientDrawable").unwrap_or(false))
        .unwrap_or(false);

    if is_gradient {
        existing.unwrap()
    } else {
        let drawable = env.new_object("android/graphics/drawable/GradientDrawable", "()V", &[]).unwrap();
        env.call_method(&drawable, "setColor", "(I)V", &[JValue::Int(0x00000000u32 as i32)]).unwrap();
        env.call_method(
            view, "setForeground", "(Landroid/graphics/drawable/Drawable;)V",
            &[JValue::Object(&drawable)],
        ).unwrap();
        drawable
    }
}

fn apply_border_radius(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else { return; };
    let drawable = get_or_create_gradient_drawable(env, view);
    env.call_method(&drawable, "setCornerRadius", "(F)V", &[JValue::Float(v as f32)]).unwrap();
}

const DEFAULT_BORDER_COLOR: i32 = 0xFF000000u32 as i32;

fn get_stroke_state(env: &mut JNIEnv, view: &JObject) -> (i32, i32) {
    let default_width = (SPACING.get(1).copied().unwrap_or(1.0) * 3.0) as i32;

    let tag = env
        .call_method(view, "getTag", "()Ljava/lang/Object;", &[])
        .ok()
        .and_then(|r| r.l().ok());

    if let Some(tag) = tag {
        if !tag.is_null() && env.is_instance_of(&tag, "[I").unwrap_or(false) {
            let arr = jni::objects::JIntArray::from(tag);
            let mut buf = [0i32; 2];
            if env.get_int_array_region(&arr, 0, &mut buf).is_ok() {
                return (buf[0], buf[1]);
            }
        }
    }

    (default_width, DEFAULT_BORDER_COLOR)
}

fn set_stroke_state(env: &mut JNIEnv, view: &JObject, width: i32, color: i32) {
    let arr = env.new_int_array(2).unwrap();
    env.set_int_array_region(&arr, 0, &[width, color]).unwrap();
    env.call_method(view, "setTag", "(Ljava/lang/Object;)V", &[JValue::Object(&arr)]).unwrap();

    let drawable = get_or_create_foreground_drawable(env, view);
    env.call_method(&drawable, "setStroke", "(II)V", &[JValue::Int(width), JValue::Int(color)]).unwrap();
}

fn apply_border_width(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(w) = resolve_length(value) else { return; };
    let (_, color) = get_stroke_state(env, view);
    set_stroke_state(env, view, w, color);
}

fn apply_border_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else { return; };
    let (width, _) = get_stroke_state(env, view);
    set_stroke_state(env, view, width, c);
}

fn apply_opacity(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Number(alpha) = value else { return; };
    env.call_method(view, "setAlpha", "(F)V", &[JValue::Float(*alpha)]).unwrap();
}

fn apply_background_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else { return; };
    let drawable = get_or_create_gradient_drawable(env, view);
    env.call_method(&drawable, "setColor", "(I)V", &[JValue::Int(c)]).unwrap();
}

fn apply_font_size(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(size) = resolve_font_size(value) else { return; };
    env.call_method(view, "setTextSize", "(F)V", &[JValue::Float(size)]).unwrap();
}

fn apply_padding_all(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else { return; };
    set_padding(env, view, v, v, v, v);
}

fn apply_padding_horizontal(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else { return; };
    let (_, t, _, b) = get_padding(env, view);
    set_padding(env, view, v, t, v, b);
}

fn apply_padding_vertical(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else { return; };
    let (l, _, r, _) = get_padding(env, view);
    set_padding(env, view, l, v, r, v);
}

fn apply_margin_all(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else { return; };

    let Ok(params) = env
        .call_method(view, "getLayoutParams", "()Landroid/view/ViewGroup$LayoutParams;", &[])
        .and_then(|r| r.l())
    else {
        return;
    };

    let is_margin_params = env
        .is_instance_of(&params, "android/view/ViewGroup$MarginLayoutParams")
        .unwrap_or(false);

    if !is_margin_params {
        return;
    }

    env.call_method(
        &params, "setMargins", "(IIII)V",
        &[JValue::Int(v), JValue::Int(v), JValue::Int(v), JValue::Int(v)],
    ).unwrap();

    env.call_method(
        view, "setLayoutParams", "(Landroid/view/ViewGroup$LayoutParams;)V",
        &[JValue::Object(&params)],
    ).unwrap();
}

fn build_style_registry() -> HashMap<Axis, StyleApplier> {
    let mut m: HashMap<Axis, StyleApplier> = HashMap::new();
    m.insert(Axis::TextColor, apply_text_color);
    m.insert(Axis::BackgroundColor, apply_background_color);
    m.insert(Axis::FontSize, apply_font_size);
    m.insert(Axis::Padding(Edge::All), apply_padding_all);
    m.insert(Axis::Padding(Edge::Horizontal), apply_padding_horizontal);
    m.insert(Axis::Padding(Edge::Vertical), apply_padding_vertical);
    m.insert(Axis::Margin(Edge::All), apply_margin_all);
    m.insert(Axis::BorderRadius, apply_border_radius);
    m.insert(Axis::BorderWidth, apply_border_width);
    m.insert(Axis::BorderColor, apply_border_color);
    m.insert(Axis::Opacity, apply_opacity);
    m
}

static STYLE_REGISTRY: Lazy<HashMap<Axis, StyleApplier>> = Lazy::new(build_style_registry);

#[derive(Clone)]
pub struct AndroidView {
    pub global_ref: Arc<GlobalRef>,
}

impl AndroidView {
    pub fn as_jobject<'a>(&self, env: &mut JNIEnv<'a>) -> JObject<'a> {
        env.new_local_ref(self.global_ref.as_obj())
            .expect("Failed to create local ref")
    }
}

#[derive(Clone)]
pub struct AndroidUpdater;

impl BackendUpdater for AndroidUpdater {
    type PlatformView = AndroidView;

    fn apply_update(&mut self, view: &Self::PlatformView, update: Update) {
        if let Some(jvm) = JVM.get() {
            if let Ok(mut env) = jvm.attach_current_thread() {
                match update {
                    Update::SetText(content) => {
                        let java_string = env.new_string(&content).unwrap();
                        let local_view = env.new_local_ref(view.global_ref.as_obj()).unwrap();
                        
                        env.call_method(
                            &local_view,
                            "setText",
                            "(Ljava/lang/CharSequence;)V",
                            &[JValue::Object(&java_string)],
                        ).unwrap();
                    }
                }
            }
        }
    }
}

pub struct AndroidBackend<'a, 'b> {
    pub env: &'a mut JNIEnv<'b>,
    pub context: &'a JObject<'b>,
}

impl<'a, 'b> AndroidBackend<'a, 'b> {
    pub fn new(env: &'a mut JNIEnv<'b>, context: &'a JObject<'b>) -> Self {
        Self { env, context }
    }
}

impl<'a, 'b> Backend for AndroidBackend<'a, 'b> {
    type PlatformView = AndroidView;
    type Updater = AndroidUpdater;

    fn clone_updater(&self) -> Self::Updater {
        AndroidUpdater
    }

    fn create_text(&mut self, content: &str) -> Self::PlatformView {
        let java_string = self.env.new_string(content).unwrap();
        let text_view = self.env
            .new_object("android/widget/TextView", "(Landroid/content/Context;)V", &[JValue::Object(self.context)]).unwrap();

        self.env.call_method(&text_view, "setText", "(Ljava/lang/CharSequence;)V", &[JValue::Object(&java_string)]).unwrap();

        let layout_params = self.env
            .new_object("android/widget/LinearLayout$LayoutParams", "(II)V", &[JValue::Int(-1), JValue::Int(-2)]).unwrap();
        self.env.call_method(&text_view, "setLayoutParams", "(Landroid/view/ViewGroup$LayoutParams;)V", &[JValue::Object(&layout_params)]).unwrap();

        AndroidView { global_ref: Arc::new(self.env.new_global_ref(text_view).unwrap()) }
    }

    fn create_button(&mut self, text: &str) -> Self::PlatformView {
        let java_string = self.env.new_string(text).unwrap();
        
        let button_view = self.env
            .new_object("android/widget/TextView", "(Landroid/content/Context;)V", &[JValue::Object(self.context)]).unwrap();

        self.env.call_method(&button_view, "setText", "(Ljava/lang/CharSequence;)V", &[JValue::Object(&java_string)]).unwrap();
        
        self.env.call_method(&button_view, "setClickable", "(Z)V", &[JValue::Bool(1)]).unwrap();
        self.env.call_method(&button_view, "setFocusable", "(Z)V", &[JValue::Bool(1)]).unwrap();
        self.env.call_method(&button_view, "setGravity", "(I)V", &[JValue::Int(17)]).unwrap();

        set_padding(self.env, &button_view, 48, 32, 48, 32);

        let layout_params = self.env
            .new_object("android/widget/LinearLayout$LayoutParams", "(II)V", &[JValue::Int(-2), JValue::Int(-2)]).unwrap();
        self.env.set_field(&layout_params, "gravity", "I", JValue::Int(1)).unwrap();
        self.env.call_method(&button_view, "setLayoutParams", "(Landroid/view/ViewGroup$LayoutParams;)V", &[JValue::Object(&layout_params)]).unwrap();

        AndroidView { global_ref: Arc::new(self.env.new_global_ref(button_view).unwrap()) }
    }

    fn create_stack(&mut self, direction: LayoutDirection, spacing: i32, children: Vec<Self::PlatformView>) -> Self::PlatformView {
        let layout = self.env
            .new_object("android/widget/LinearLayout", "(Landroid/content/Context;)V", &[JValue::Object(self.context)]).unwrap();

        let orientation = match direction {
            LayoutDirection::Horizontal => 0, 
            LayoutDirection::Vertical => 1,   
        };
        self.env.call_method(&layout, "setOrientation", "(I)V", &[JValue::Int(orientation)]).unwrap();

        let real_spacing = (spacing * 3) as i32;

        for (idx, child_view) in children.into_iter().enumerate() {
            let local_child = self.env.new_local_ref(child_view.global_ref.as_obj()).unwrap();
            
            let layout_params = self.env
                .new_object("android/widget/LinearLayout$LayoutParams", "(II)V", &[JValue::Int(-2), JValue::Int(-2)]).unwrap();
            
            self.env.set_field(&layout_params, "gravity", "I", JValue::Int(1)).unwrap();

            if spacing > 0 && idx > 0 {
                let (margin_l, margin_t) = if direction == LayoutDirection::Vertical {
                    (0, real_spacing)
                } else {
                    (real_spacing, 0)
                };

                self.env.call_method(
                    &layout_params, 
                    "setMargins", 
                    "(IIII)V", 
                    &[JValue::Int(margin_l), JValue::Int(margin_t), JValue::Int(0), JValue::Int(0)]
                ).unwrap();
            }

            self.env.call_method(
                &layout,
                "addView",
                "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
                &[JValue::Object(&local_child), JValue::Object(&layout_params)],
            ).unwrap();
        }

        self.env.call_method(&layout, "requestLayout", "()V", &[]).unwrap();

        let layout_params = self.env
            .new_object("android/widget/LinearLayout$LayoutParams", "(II)V", &[JValue::Int(-1), JValue::Int(-1)]).unwrap();
        self.env.call_method(&layout, "setLayoutParams", "(Landroid/view/ViewGroup$LayoutParams;)V", &[JValue::Object(&layout_params)]).unwrap();

        AndroidView { global_ref: Arc::new(self.env.new_global_ref(layout).unwrap()) }
    }

    fn apply_style(&mut self, view: &Self::PlatformView, style: StyleProperty) {
        let local_view = self.env.new_local_ref(view.global_ref.as_obj()).unwrap();
        let StyleProperty(axis, value) = style;

        match STYLE_REGISTRY.get(&axis) {
            Some(applier) => applier(&mut *self.env, &local_view, &value),
            None => {
                #[cfg(debug_assertions)]
                eprintln!("goyda(android): axis {axis:?} is not supported on this backend, skipped");
            }
        }
    }
}
