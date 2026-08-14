use crate::components::{
    Align, Asset, Axis, Color, Edge, LayoutDirection, StyleProperty, StyleValue,
};
use crate::core::events::{Event, Update};
use crate::core::{Backend, BackendUpdater};
use goyda_macros::{jcall, jnew, new_widget, set_layout_params, sig};
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use once_cell::sync::{Lazy, OnceCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// The Android app's Java VM handle, set once at startup.
pub static JVM: OnceCell<JavaVM> = OnceCell::new();

static RADIO_GROUPS: Lazy<Mutex<HashMap<String, Vec<Arc<GlobalRef>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn register_radio(group: &str, view: &AndroidView) {
    RADIO_GROUPS
        .lock()
        .unwrap()
        .entry(group.to_string())
        .or_default()
        .push(view.global_ref.clone());
}

fn select_radio(env: &mut JNIEnv, group: &str, selected: &AndroidView) {
    let members = RADIO_GROUPS
        .lock()
        .unwrap()
        .get(group)
        .cloned()
        .unwrap_or_default();
    for member in &members {
        let is_selected = Arc::ptr_eq(member, &selected.global_ref);
        let local = env.new_local_ref(member.as_obj()).unwrap();
        let _ = env.call_method(
            &local,
            "setChecked",
            sig!((boolean) -> void),
            &[JValue::Bool(is_selected as u8)],
        );
    }
}

type StyleApplier = fn(&mut JNIEnv, &JObject, &StyleValue);

fn map_color(color: Color) -> i32 {
    goyda_utils::color::argb(color) as i32
}

fn resolve_color(value: &StyleValue) -> Option<i32> {
    match value {
        StyleValue::Color(c) => Some(map_color(*c)),
        _ => None,
    }
}

fn resolve_length(value: &StyleValue) -> Option<i32> {
    let v = goyda_utils::style::resolve_length(value);
    if v.is_none() {
        if let StyleValue::Spacing(_scale) = value {
            #[cfg(debug_assertions)]
            eprintln!("goyda(android): spacing scale index {_scale} out of range");
        }
    }
    v.map(|v| (v * 3.0) as i32)
}

fn resolve_font_size(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Length(v) => Some(*v),
        _ => None,
    }
}

fn get_padding(env: &mut JNIEnv, view: &JObject) -> (i32, i32, i32, i32) {
    let l = env
        .call_method(view, "getPaddingLeft", sig!(() -> int), &[])
        .unwrap()
        .i()
        .unwrap();
    let t = env
        .call_method(view, "getPaddingTop", sig!(() -> int), &[])
        .unwrap()
        .i()
        .unwrap();
    let r = env
        .call_method(view, "getPaddingRight", sig!(() -> int), &[])
        .unwrap()
        .i()
        .unwrap();
    let b = env
        .call_method(view, "getPaddingBottom", sig!(() -> int), &[])
        .unwrap()
        .i()
        .unwrap();
    (l, t, r, b)
}

fn set_padding(env: &mut JNIEnv, view: &JObject, l: i32, t: i32, r: i32, b: i32) {
    env.call_method(
        view,
        "setPadding",
        sig!((int, int, int, int) -> void),
        &[
            JValue::Int(l),
            JValue::Int(t),
            JValue::Int(r),
            JValue::Int(b),
        ],
    )
    .unwrap();
}

fn apply_text_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else {
        return;
    };
    env.call_method(view, "setTextColor", sig!((int) -> void), &[JValue::Int(c)])
        .unwrap();
}

const DEFAULT_TEXT_COLOR: i32 = 0xFF00_0000u32 as i32;

fn set_default_text_color(env: &mut JNIEnv, view: &JObject) {
    env.call_method(
        view,
        "setTextColor",
        sig!((int) -> void),
        &[JValue::Int(DEFAULT_TEXT_COLOR)],
    )
    .unwrap();
}

fn get_or_create_gradient_drawable<'a>(env: &mut JNIEnv<'a>, view: &JObject) -> JObject<'a> {
    let existing = env
        .call_method(
            view,
            "getBackground",
            sig!(() -> "android/graphics/drawable/Drawable"),
            &[],
        )
        .ok()
        .and_then(|r| r.l().ok());

    let is_gradient = existing
        .as_ref()
        .map(|d| {
            !d.is_null()
                && env
                    .is_instance_of(d, "android/graphics/drawable/GradientDrawable")
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    if is_gradient {
        existing.unwrap()
    } else {
        let (l, t, r, b) = get_padding(env, view);

        let drawable = env
            .new_object(
                "android/graphics/drawable/GradientDrawable",
                sig!(() -> void),
                &[],
            )
            .unwrap();
        env.call_method(
            view,
            "setBackground",
            sig!(("android/graphics/drawable/Drawable") -> void),
            &[JValue::Object(&drawable)],
        )
        .unwrap();

        let is_clickable = env
            .call_method(view, "isClickable", sig!(() -> boolean), &[])
            .unwrap()
            .z()
            .unwrap();
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
        .call_method(
            view,
            "getForeground",
            sig!(() -> "android/graphics/drawable/Drawable"),
            &[],
        )
        .ok()
        .and_then(|r| r.l().ok());

    let is_gradient = existing
        .as_ref()
        .map(|d| {
            !d.is_null()
                && env
                    .is_instance_of(d, "android/graphics/drawable/GradientDrawable")
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    if is_gradient {
        existing.unwrap()
    } else {
        let drawable = env
            .new_object(
                "android/graphics/drawable/GradientDrawable",
                sig!(() -> void),
                &[],
            )
            .unwrap();
        env.call_method(
            &drawable,
            "setColor",
            sig!((int) -> void),
            &[JValue::Int(0x00000000u32 as i32)],
        )
        .unwrap();
        env.call_method(
            view,
            "setForeground",
            sig!(("android/graphics/drawable/Drawable") -> void),
            &[JValue::Object(&drawable)],
        )
        .unwrap();
        drawable
    }
}

fn apply_border_radius(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    let drawable = get_or_create_gradient_drawable(env, view);
    env.call_method(
        &drawable,
        "setCornerRadius",
        sig!((float) -> void),
        &[JValue::Float(v as f32)],
    )
    .unwrap();
}

const DEFAULT_BORDER_COLOR: i32 = 0xFF000000u32 as i32;

fn get_stroke_state(env: &mut JNIEnv, view: &JObject) -> (i32, i32) {
    let default_width = (goyda_utils::style::resolve_spacing(1).unwrap_or(1.0) * 3.0) as i32;

    let tag = env
        .call_method(view, "getTag", sig!(() -> "java/lang/Object"), &[])
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
    env.call_method(
        view,
        "setTag",
        sig!(("java/lang/Object") -> void),
        &[JValue::Object(&arr)],
    )
    .unwrap();

    let drawable = get_or_create_foreground_drawable(env, view);
    env.call_method(
        &drawable,
        "setStroke",
        sig!((int, int) -> void),
        &[JValue::Int(width), JValue::Int(color)],
    )
    .unwrap();
}

fn apply_border_width(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(w) = resolve_length(value) else {
        return;
    };
    let (_, color) = get_stroke_state(env, view);
    set_stroke_state(env, view, w, color);
}

fn apply_border_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else {
        return;
    };
    let (width, _) = get_stroke_state(env, view);
    set_stroke_state(env, view, width, c);
}

fn apply_opacity(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Number(alpha) = value else {
        return;
    };
    env.call_method(
        view,
        "setAlpha",
        sig!((float) -> void),
        &[JValue::Float(*alpha)],
    )
    .unwrap();
}

fn apply_background_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else {
        return;
    };
    let drawable = get_or_create_gradient_drawable(env, view);
    env.call_method(
        &drawable,
        "setColor",
        sig!((int) -> void),
        &[JValue::Int(c)],
    )
    .unwrap();
}

fn apply_font_size(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(size) = resolve_font_size(value) else {
        return;
    };
    env.call_method(
        view,
        "setTextSize",
        sig!((float) -> void),
        &[JValue::Float(size)],
    )
    .unwrap();
}

fn apply_padding_all(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    set_padding(env, view, v, v, v, v);
}

fn apply_padding_horizontal(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    let (_, t, _, b) = get_padding(env, view);
    set_padding(env, view, v, t, v, b);
}

fn apply_padding_vertical(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    let (l, _, r, _) = get_padding(env, view);
    set_padding(env, view, l, v, r, v);
}

fn apply_margin_all(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };

    let Ok(params) = env
        .call_method(
            view,
            "getLayoutParams",
            sig!(() -> "android/view/ViewGroup$LayoutParams"),
            &[],
        )
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
        &params,
        "setMargins",
        sig!((int, int, int, int) -> void),
        &[
            JValue::Int(v),
            JValue::Int(v),
            JValue::Int(v),
            JValue::Int(v),
        ],
    )
    .unwrap();

    env.call_method(
        view,
        "setLayoutParams",
        sig!(("android/view/ViewGroup$LayoutParams") -> void),
        &[JValue::Object(&params)],
    )
    .unwrap();
}

fn set_layout_dimension(env: &mut JNIEnv, view: &JObject, field: &str, v: i32) {
    let Ok(params) = env
        .call_method(
            view,
            "getLayoutParams",
            sig!(() -> "android/view/ViewGroup$LayoutParams"),
            &[],
        )
        .and_then(|r| r.l())
    else {
        return;
    };
    let _ = env.set_field(&params, field, "I", JValue::Int(v));
    let _ = env.call_method(
        view,
        "setLayoutParams",
        sig!(("android/view/ViewGroup$LayoutParams") -> void),
        &[JValue::Object(&params)],
    );
}

fn apply_width(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    set_layout_dimension(env, view, "width", v);
}

fn apply_height(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    set_layout_dimension(env, view, "height", v);
}

fn current_typeface_style(env: &mut JNIEnv, view: &JObject) -> i32 {
    env.call_method(
        view,
        "getTypeface",
        sig!(() -> "android/graphics/Typeface"),
        &[],
    )
    .ok()
    .and_then(|r| r.l().ok())
    .filter(|t| !t.is_null())
    .and_then(|t| env.call_method(&t, "getStyle", sig!(() -> int), &[]).ok())
    .and_then(|r| r.i().ok())
    .unwrap_or(0)
}

fn set_typeface_style(env: &mut JNIEnv, view: &JObject, style: i32) {
    let _ = env.call_method(
        view,
        "setTypeface",
        sig!(("android/graphics/Typeface", int) -> void),
        &[JValue::Object(&JObject::null()), JValue::Int(style)],
    );
}

const TYPEFACE_BOLD: i32 = 1;
const TYPEFACE_ITALIC: i32 = 2;

fn apply_font_weight(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(bold) = value else {
        return;
    };
    let current = current_typeface_style(env, view);
    let updated = if *bold {
        current | TYPEFACE_BOLD
    } else {
        current & !TYPEFACE_BOLD
    };
    set_typeface_style(env, view, updated);
}

fn apply_font_style(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(italic) = value else {
        return;
    };
    let current = current_typeface_style(env, view);
    let updated = if *italic {
        current | TYPEFACE_ITALIC
    } else {
        current & !TYPEFACE_ITALIC
    };
    set_typeface_style(env, view, updated);
}

const GRAVITY_LEFT: i32 = 3;
const GRAVITY_CENTER_HORIZONTAL: i32 = 1;
const GRAVITY_RIGHT: i32 = 5;
const GRAVITY_FILL_HORIZONTAL: i32 = 7;
const GRAVITY_TOP: i32 = 48;
const GRAVITY_CENTER_VERTICAL: i32 = 16;
const GRAVITY_BOTTOM: i32 = 80;
const GRAVITY_FILL_VERTICAL: i32 = 112;
const GRAVITY_HORIZONTAL_MASK: i32 = 7;
const GRAVITY_VERTICAL_MASK: i32 = 112;

fn apply_text_align(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Align(align) = value else {
        return;
    };
    let gravity = match align {
        Align::Start => GRAVITY_LEFT,
        Align::Center => GRAVITY_CENTER_HORIZONTAL,
        Align::End => GRAVITY_RIGHT,
        Align::Stretch | Align::SpaceBetween => GRAVITY_LEFT,
    };
    let _ = env.call_method(
        view,
        "setGravity",
        sig!((int) -> void),
        &[JValue::Int(gravity)],
    );
}

fn horizontal_gravity(align: Align) -> i32 {
    match align {
        Align::Start => GRAVITY_LEFT,
        Align::Center => GRAVITY_CENTER_HORIZONTAL,
        Align::End => GRAVITY_RIGHT,
        Align::Stretch => GRAVITY_FILL_HORIZONTAL,
        // `LinearLayout` has no native "even spacing" gravity - centering is
        // the closest single-value approximation.
        Align::SpaceBetween => GRAVITY_CENTER_HORIZONTAL,
    }
}

fn vertical_gravity(align: Align) -> i32 {
    match align {
        Align::Start => GRAVITY_TOP,
        Align::Center => GRAVITY_CENTER_VERTICAL,
        Align::End => GRAVITY_BOTTOM,
        Align::Stretch => GRAVITY_FILL_VERTICAL,
        Align::SpaceBetween => GRAVITY_CENTER_VERTICAL,
    }
}

fn stack_orientation(env: &mut JNIEnv, view: &JObject) -> Option<i32> {
    env.call_method(view, "getOrientation", sig!(() -> int), &[])
        .ok()?
        .i()
        .ok()
}

fn current_gravity(env: &mut JNIEnv, view: &JObject) -> i32 {
    env.call_method(view, "getGravity", sig!(() -> int), &[])
        .ok()
        .and_then(|r| r.i().ok())
        .unwrap_or(0)
}

fn apply_align_items(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Align(align) = value else {
        return;
    };
    let Some(orientation) = stack_orientation(env, view) else {
        return;
    };
    let current = current_gravity(env, view);
    let updated = if orientation == 1 {
        (current & !GRAVITY_HORIZONTAL_MASK) | horizontal_gravity(*align)
    } else {
        (current & !GRAVITY_VERTICAL_MASK) | vertical_gravity(*align)
    };
    let _ = env.call_method(
        view,
        "setGravity",
        sig!((int) -> void),
        &[JValue::Int(updated)],
    );
}

fn apply_justify_content(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Align(align) = value else {
        return;
    };
    let Some(orientation) = stack_orientation(env, view) else {
        return;
    };
    let current = current_gravity(env, view);
    let updated = if orientation == 1 {
        (current & !GRAVITY_VERTICAL_MASK) | vertical_gravity(*align)
    } else {
        (current & !GRAVITY_HORIZONTAL_MASK) | horizontal_gravity(*align)
    };
    let _ = env.call_method(
        view,
        "setGravity",
        sig!((int) -> void),
        &[JValue::Int(updated)],
    );
}

fn read_asset_bytes(env: &mut JNIEnv, context: &JObject, path: &str) -> Option<Vec<u8>> {
    let asset_manager = env
        .call_method(
            context,
            "getAssets",
            sig!(() -> "android/content/res/AssetManager"),
            &[],
        )
        .ok()?
        .l()
        .ok()?;

    let path_str = env.new_string(path).ok()?;
    let input_stream = env.call_method(
        &asset_manager,
        "open",
        sig!(("java/lang/String") -> "java/io/InputStream"),
        &[JValue::Object(&path_str)],
    );

    let input_stream = match input_stream {
        Ok(v) => v.l().ok()?,
        Err(_) => {
            let _ = env.exception_clear();
            return None;
        }
    };

    let available = env
        .call_method(&input_stream, "available", sig!(() -> int), &[])
        .ok()?
        .i()
        .ok()?;
    if available <= 0 {
        return None;
    }

    let byte_array = env.new_byte_array(available).ok()?;
    let read = env
        .call_method(
            &input_stream,
            "read",
            sig!(([byte]) -> int),
            &[JValue::Object(&byte_array)],
        )
        .ok()?
        .i()
        .ok()?;
    if read <= 0 {
        return None;
    }

    let _ = env.call_method(&input_stream, "close", sig!(() -> void), &[]);

    let mut buf = vec![0i8; read as usize];
    env.get_byte_array_region(&byte_array, 0, &mut buf).ok()?;
    Some(buf.into_iter().map(|b| b as u8).collect())
}

fn rasterize_svg(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let (width, height) = (size.width().ceil() as u32, size.height().ceil() as u32);
    if width == 0 || height == 0 {
        return None;
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    Some((width, height, pixmap.data().to_vec()))
}

fn create_bitmap_from_rgba<'a>(
    env: &mut JNIEnv<'a>,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Option<JObject<'a>> {
    let config_class = env.find_class("android/graphics/Bitmap$Config").ok()?;
    let argb_8888 = env
        .get_static_field(
            config_class,
            "ARGB_8888",
            "Landroid/graphics/Bitmap$Config;",
        )
        .ok()?
        .l()
        .ok()?;

    let bitmap_class = env.find_class("android/graphics/Bitmap").ok()?;
    let bitmap = env
        .call_static_method(
            bitmap_class,
            "createBitmap",
            sig!((int, int, "android/graphics/Bitmap$Config") -> "android/graphics/Bitmap"),
            &[
                JValue::Int(width as i32),
                JValue::Int(height as i32),
                JValue::Object(&argb_8888),
            ],
        )
        .ok()?
        .l()
        .ok()?;

    let byte_array = env.byte_array_from_slice(rgba).ok()?;
    let buffer_class = env.find_class("java/nio/ByteBuffer").ok()?;
    let buffer = env
        .call_static_method(
            buffer_class,
            "wrap",
            sig!(([byte]) -> "java/nio/ByteBuffer"),
            &[JValue::Object(&byte_array)],
        )
        .ok()?
        .l()
        .ok()?;

    env.call_method(
        &bitmap,
        "copyPixelsFromBuffer",
        sig!(("java/nio/Buffer") -> void),
        &[JValue::Object(&buffer)],
    )
    .ok()?;

    Some(bitmap)
}

fn decode_bitmap_bytes<'a>(
    env: &mut JNIEnv<'a>,
    asset: &Asset,
    bytes: &[u8],
) -> Option<JObject<'a>> {
    if goyda_utils::asset::is_svg(asset) {
        let (width, height, rgba) = rasterize_svg(bytes)?;
        return create_bitmap_from_rgba(env, width, height, &rgba);
    }

    let array = env.byte_array_from_slice(bytes).ok()?;
    let bitmap_factory_class = env.find_class("android/graphics/BitmapFactory").ok()?;
    env.call_static_method(
        bitmap_factory_class,
        "decodeByteArray",
        sig!(([byte], int, int) -> "android/graphics/Bitmap"),
        &[
            JValue::Object(&array),
            JValue::Int(0),
            JValue::Int(bytes.len() as i32),
        ],
    )
    .ok()?
    .l()
    .ok()
}

fn load_asset_bitmap<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject<'a>,
    asset: &Asset,
) -> Option<JObject<'a>> {
    let bytes = read_asset_bytes(env, context, asset.path())?;
    decode_bitmap_bytes(env, asset, &bytes)
}

fn build_typeface_bytes<'a>(env: &mut JNIEnv<'a>, bytes: &[u8]) -> Option<JObject<'a>> {
    let array = env.byte_array_from_slice(bytes).ok()?;
    let builder = env
        .new_object(
            "android/graphics/Typeface$Builder",
            sig!(([byte]) -> void),
            &[JValue::Object(&array)],
        )
        .ok()?;
    env.call_method(
        &builder,
        "build",
        sig!(() -> "android/graphics/Typeface"),
        &[],
    )
    .ok()?
    .l()
    .ok()
}

fn apply_font_family<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject<'a>,
    view: &JObject<'a>,
    asset: &Asset,
) {
    let typeface = if let Some(bytes) = asset.bytes() {
        build_typeface_bytes(env, bytes)
    } else {
        load_asset_typeface(env, context, asset.path())
    };

    let Some(typeface) = typeface else {
        #[cfg(debug_assertions)]
        eprintln!("goyda(android): font asset not found: {}", asset.path());
        return;
    };

    let _ = env.call_method(
        view,
        "setTypeface",
        sig!(("android/graphics/Typeface") -> void),
        &[JValue::Object(&typeface)],
    );
}

fn load_asset_typeface<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject<'a>,
    path: &str,
) -> Option<JObject<'a>> {
    let asset_manager = env
        .call_method(
            context,
            "getAssets",
            sig!(() -> "android/content/res/AssetManager"),
            &[],
        )
        .ok()?
        .l()
        .ok()?;

    let path_str = env.new_string(path).ok()?;
    let typeface_class = env.find_class("android/graphics/Typeface").ok()?;

    let typeface = env.call_static_method(
        typeface_class,
        "createFromAsset",
        sig!(("android/content/res/AssetManager", "java/lang/String") -> "android/graphics/Typeface"),
        &[JValue::Object(&asset_manager), JValue::Object(&path_str)],
    );

    match typeface.and_then(|v| v.l()) {
        Ok(t) => Some(t),
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    }
}

fn apply_line_height(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    // `setLineHeight(int)` (an *absolute* px line height) only exists from
    // API 28 - `setLineSpacing(extra, mult)` is the portable fallback,
    // added as extra space per line rather than a true absolute height.
    let _ = env.call_method(
        view,
        "setLineSpacing",
        sig!((float, float) -> void),
        &[JValue::Float(v as f32), JValue::Float(1.0)],
    );
}

fn apply_letter_spacing(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    // `setLetterSpacing` takes EM units, not px - convert using the view's
    // own current text size.
    let text_size = env
        .call_method(view, "getTextSize", sig!(() -> float), &[])
        .ok()
        .and_then(|r| r.f().ok())
        .unwrap_or(1.0)
        .max(1.0);
    let _ = env.call_method(
        view,
        "setLetterSpacing",
        sig!((float) -> void),
        &[JValue::Float(v as f32 / text_size)],
    );
}

const PAINT_UNDERLINE_FLAG: i32 = 8;
const PAINT_STRIKETHRU_FLAG: i32 = 16;

fn current_paint_flags(env: &mut JNIEnv, view: &JObject) -> i32 {
    env.call_method(view, "getPaintFlags", sig!(() -> int), &[])
        .ok()
        .and_then(|r| r.i().ok())
        .unwrap_or(0)
}

fn apply_underline(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(on) = value else {
        return;
    };
    let current = current_paint_flags(env, view);
    let updated = if *on {
        current | PAINT_UNDERLINE_FLAG
    } else {
        current & !PAINT_UNDERLINE_FLAG
    };
    let _ = env.call_method(
        view,
        "setPaintFlags",
        sig!((int) -> void),
        &[JValue::Int(updated)],
    );
}

fn apply_strikethrough(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(on) = value else {
        return;
    };
    let current = current_paint_flags(env, view);
    let updated = if *on {
        current | PAINT_STRIKETHRU_FLAG
    } else {
        current & !PAINT_STRIKETHRU_FLAG
    };
    let _ = env.call_method(
        view,
        "setPaintFlags",
        sig!((int) -> void),
        &[JValue::Int(updated)],
    );
}

fn apply_ellipsis(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(true) = value else {
        return;
    };
    let Ok(truncate_class) = env.find_class("android/text/TextUtils$TruncateAt") else {
        return;
    };
    let Ok(end) =
        env.get_static_field(truncate_class, "END", "Landroid/text/TextUtils$TruncateAt;")
    else {
        return;
    };
    let Ok(end_obj) = end.l() else {
        return;
    };
    let _ = env.call_method(
        view,
        "setSingleLine",
        sig!((boolean) -> void),
        &[JValue::Bool(1)],
    );
    let _ = env.call_method(
        view,
        "setEllipsize",
        sig!(("android/text/TextUtils$TruncateAt") -> void),
        &[JValue::Object(&end_obj)],
    );
}

fn apply_clip(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let StyleValue::Bool(clip) = value else {
        return;
    };
    let _ = env.call_method(
        view,
        "setClipChildren",
        sig!((boolean) -> void),
        &[JValue::Bool(*clip as u8)],
    );
    let _ = env.call_method(
        view,
        "setClipToPadding",
        sig!((boolean) -> void),
        &[JValue::Bool(*clip as u8)],
    );
}

fn apply_shadow(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    // `elevation` (API 21+) is the only cross-version native shadow a
    // plain `View` has - it's a soft drop shadow shaped by the view's own
    // outline, not a configurable offset/blur/color like the other two
    // backends' hand-painted shadow, so size is the only knob that maps
    // cleanly (see `apply_shadow_color` for the color's own limits).
    let _ = env.call_method(
        view,
        "setElevation",
        sig!((float) -> void),
        &[JValue::Float(v as f32)],
    );
}

fn apply_shadow_color(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(c) = resolve_color(value) else {
        return;
    };
    // API 28+ only (`setOutlineSpotShadowColor`) - silently a no-op on
    // older devices, same as every other elevation-shadow color knob on
    // Android.
    let _ = env.call_method(
        view,
        "setOutlineSpotShadowColor",
        sig!((int) -> void),
        &[JValue::Int(c)],
    );
    let _ = env.call_method(
        view,
        "setOutlineAmbientShadowColor",
        sig!((int) -> void),
        &[JValue::Int(c)],
    );
}

fn apply_offset_x(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    set_layout_margin(env, view, "leftMargin", v);
}

fn apply_offset_y(env: &mut JNIEnv, view: &JObject, value: &StyleValue) {
    let Some(v) = resolve_length(value) else {
        return;
    };
    set_layout_margin(env, view, "topMargin", v);
}

fn set_layout_margin(env: &mut JNIEnv, view: &JObject, field: &str, v: i32) {
    let Ok(params) = env
        .call_method(
            view,
            "getLayoutParams",
            sig!(() -> "android/view/ViewGroup$LayoutParams"),
            &[],
        )
        .and_then(|r| r.l())
    else {
        return;
    };
    if !env
        .is_instance_of(&params, "android/view/ViewGroup$MarginLayoutParams")
        .unwrap_or(false)
    {
        return;
    }
    let _ = env.set_field(&params, field, "I", JValue::Int(v));
    let _ = env.call_method(
        view,
        "setLayoutParams",
        sig!(("android/view/ViewGroup$LayoutParams") -> void),
        &[JValue::Object(&params)],
    );
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
    m.insert(Axis::Width, apply_width);
    m.insert(Axis::Height, apply_height);
    m.insert(Axis::FontWeight, apply_font_weight);
    m.insert(Axis::FontStyle, apply_font_style);
    m.insert(Axis::TextAlign, apply_text_align);
    m.insert(Axis::AlignItems, apply_align_items);
    m.insert(Axis::JustifyContent, apply_justify_content);
    m.insert(Axis::LineHeight, apply_line_height);
    m.insert(Axis::LetterSpacing, apply_letter_spacing);
    m.insert(Axis::Underline, apply_underline);
    m.insert(Axis::Strikethrough, apply_strikethrough);
    m.insert(Axis::TextOverflowEllipsis, apply_ellipsis);
    m.insert(Axis::Clip, apply_clip);
    m.insert(Axis::Shadow, apply_shadow);
    m.insert(Axis::ShadowColor, apply_shadow_color);
    m.insert(Axis::OffsetX, apply_offset_x);
    m.insert(Axis::OffsetY, apply_offset_y);
    m
}

static STYLE_REGISTRY: Lazy<HashMap<Axis, StyleApplier>> = Lazy::new(build_style_registry);

/// A handle to a mounted view on Android.
#[derive(Clone)]
pub struct AndroidView {
    pub global_ref: Arc<GlobalRef>,
}

impl AndroidView {
    /// Returns a local JNI reference to the underlying `View`.
    pub fn as_jobject<'a>(&self, env: &mut JNIEnv<'a>) -> JObject<'a> {
        env.new_local_ref(self.global_ref.as_obj())
            .expect("Failed to create local ref")
    }
}

/// Applies reactive updates to views on Android.
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

                        jcall!(&mut env, &local_view, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_string)]).unwrap();
                    }
                    Update::SetProgress(value) => {
                        let local_view = env.new_local_ref(view.global_ref.as_obj()).unwrap();
                        let percent = (value.clamp(0.0, 1.0) * 100.0) as i32;
                        jcall!(&mut env, &local_view, "setProgress", ((int) -> void), [JValue::Int(percent)]).unwrap();
                    }
                }
            }
        }
    }
}

/// The Android rendering backend, mounting components as native `View`s.
pub struct AndroidBackend<'a, 'b> {
    pub env: &'a mut JNIEnv<'b>,
    pub context: &'a JObject<'b>,
}

impl<'a, 'b> AndroidBackend<'a, 'b> {
    /// Creates a backend that mounts views against `context`.
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
        let text_view = new_widget!(self.env, "android/widget/TextView", self.context);

        jcall!(self.env, &text_view, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_string)]).unwrap();
        set_default_text_color(self.env, &text_view);

        set_layout_params!(self.env, &text_view, -1, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(text_view).unwrap()),
        }
    }

    fn create_button(&mut self, text: &str) -> Self::PlatformView {
        let java_string = self.env.new_string(text).unwrap();

        let button_view = new_widget!(self.env, "android/widget/TextView", self.context);

        jcall!(self.env, &button_view, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_string)]).unwrap();
        set_default_text_color(self.env, &button_view);

        jcall!(self.env, &button_view, "setClickable", ((boolean) -> void), [JValue::Bool(1)])
            .unwrap();
        jcall!(self.env, &button_view, "setFocusable", ((boolean) -> void), [JValue::Bool(1)])
            .unwrap();
        jcall!(self.env, &button_view, "setGravity", ((int) -> void), [JValue::Int(17)]).unwrap();

        set_padding(self.env, &button_view, 48, 32, 48, 32);

        let layout_params = jnew!(self.env, "android/widget/LinearLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-2), JValue::Int(-2)]);
        self.env
            .set_field(&layout_params, "gravity", "I", JValue::Int(1))
            .unwrap();
        jcall!(self.env, &button_view, "setLayoutParams", (("android/view/ViewGroup$LayoutParams") -> void), [JValue::Object(&layout_params)]).unwrap();

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(button_view).unwrap()),
        }
    }

    fn create_image(&mut self, asset: &Asset) -> Self::PlatformView {
        let image_view = new_widget!(self.env, "android/widget/ImageView", self.context);

        let bitmap = match asset.bytes() {
            Some(bytes) => decode_bitmap_bytes(self.env, asset, bytes),
            None => load_asset_bitmap(self.env, self.context, asset),
        };

        match bitmap {
            Some(bitmap) => {
                jcall!(self.env, &image_view, "setImageBitmap", (("android/graphics/Bitmap") -> void), [JValue::Object(&bitmap)]).unwrap();
            }
            None => {
                #[cfg(debug_assertions)]
                eprintln!("goyda(android): image asset not found: {}", asset.path());
            }
        }

        set_layout_params!(self.env, &image_view, -2, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(image_view).unwrap()),
        }
    }

    fn create_text_input(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        let edit_text = new_widget!(self.env, "android/widget/EditText", self.context);
        set_default_text_color(self.env, &edit_text);
        jcall!(self.env, &edit_text, "setHintTextColor", ((int) -> void), [JValue::Int(0xFF80_8080u32 as i32)]).unwrap();

        if !placeholder.is_empty() {
            let java_placeholder = self.env.new_string(placeholder).unwrap();
            jcall!(self.env, &edit_text, "setHint", (("java/lang/CharSequence") -> void), [JValue::Object(&java_placeholder)]).unwrap();
        }
        if !initial_text.is_empty() {
            let java_text = self.env.new_string(initial_text).unwrap();
            jcall!(self.env, &edit_text, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_text)]).unwrap();
        }

        set_layout_params!(self.env, &edit_text, -1, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(edit_text).unwrap()),
        }
    }

    fn create_textarea(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        let edit_text = new_widget!(self.env, "android/widget/EditText", self.context);
        set_default_text_color(self.env, &edit_text);
        jcall!(self.env, &edit_text, "setHintTextColor", ((int) -> void), [JValue::Int(0xFF80_8080u32 as i32)]).unwrap();

        // `TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE` (0x1 | 0x20000) -
        // without the multi-line flag, `EditText` treats Enter as "submit"
        // (calls the IME action) instead of inserting a newline.
        const INPUT_TYPE_TEXT_MULTILINE: i32 = 0x1 | 0x20000;
        jcall!(self.env, &edit_text, "setInputType", ((int) -> void), [JValue::Int(INPUT_TYPE_TEXT_MULTILINE)]).unwrap();
        jcall!(self.env, &edit_text, "setMinLines", ((int) -> void), [JValue::Int(4)]).unwrap();
        jcall!(self.env, &edit_text, "setGravity", ((int) -> void), [JValue::Int(48)]).unwrap(); // Gravity.TOP

        if !placeholder.is_empty() {
            let java_placeholder = self.env.new_string(placeholder).unwrap();
            jcall!(self.env, &edit_text, "setHint", (("java/lang/CharSequence") -> void), [JValue::Object(&java_placeholder)]).unwrap();
        }
        if !initial_text.is_empty() {
            let java_text = self.env.new_string(initial_text).unwrap();
            jcall!(self.env, &edit_text, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_text)]).unwrap();
        }

        set_layout_params!(self.env, &edit_text, -1, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(edit_text).unwrap()),
        }
    }

    fn create_checkbox(&mut self, label: &str, checked: bool) -> Self::PlatformView {
        let checkbox = new_widget!(self.env, "android/widget/CheckBox", self.context);

        let java_label = self.env.new_string(label).unwrap();
        jcall!(self.env, &checkbox, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_label)]).unwrap();
        set_default_text_color(self.env, &checkbox);
        jcall!(self.env, &checkbox, "setChecked", ((boolean) -> void), [JValue::Bool(checked as u8)]).unwrap();

        set_layout_params!(self.env, &checkbox, -2, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(checkbox).unwrap()),
        }
    }

    fn create_radio_button(
        &mut self,
        group: &str,
        label: &str,
        selected: bool,
    ) -> Self::PlatformView {
        let radio = new_widget!(self.env, "android/widget/RadioButton", self.context);

        let java_label = self.env.new_string(label).unwrap();
        jcall!(self.env, &radio, "setText", (("java/lang/CharSequence") -> void), [JValue::Object(&java_label)]).unwrap();
        set_default_text_color(self.env, &radio);
        jcall!(self.env, &radio, "setChecked", ((boolean) -> void), [JValue::Bool(selected as u8)])
            .unwrap();

        set_layout_params!(self.env, &radio, -2, -2);

        let view = AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(radio).unwrap()),
        };
        register_radio(group, &view);
        if selected {
            select_radio(self.env, group, &view);
        }

        // Standalone `RadioButton`s (no `RadioGroup` parent) still toggle
        // like a checkbox on click by default, which would let tapping an
        // already-selected one deselect itself - `setChecked` on click
        // (rather than `toggle`) plus `select_radio`'s group-wide
        // read-modify-write is what keeps this an actual radio: exactly one
        // member selected, chosen by clicking, never by unclicking.
        let group_owned = group.to_string();
        let target = view.global_ref.clone();
        let callback: Rc<dyn Fn(Event)> = Rc::new(move |_e| {
            if let Some(jvm) = JVM.get() {
                if let Ok(mut env) = jvm.attach_current_thread() {
                    let target_view = AndroidView {
                        global_ref: target.clone(),
                    };
                    select_radio(&mut env, &group_owned, &target_view);
                }
            }
        });
        unsafe {
            crate::listeners::on_click::attach(self, &view, callback);
        }

        view
    }

    fn create_switch(&mut self, checked: bool) -> Self::PlatformView {
        let switch = new_widget!(self.env, "android/widget/Switch", self.context);

        jcall!(self.env, &switch, "setChecked", ((boolean) -> void), [JValue::Bool(checked as u8)])
            .unwrap();

        set_layout_params!(self.env, &switch, -2, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(switch).unwrap()),
        }
    }

    fn create_progress(&mut self, value: f32) -> Self::PlatformView {
        // A scrubber, not just a read-only indicator - `SeekBar` gives
        // touch-to-seek/drag for free (report changes to the app with
        // `.on_value_changed(...)`, wired to `setOnSeekBarChangeListener` by
        // the `seek` listener - see `crate::listeners`), which a plain
        // `ProgressBar` doesn't support.
        let seek_bar = new_widget!(self.env, "android/widget/SeekBar", self.context);

        jcall!(self.env, &seek_bar, "setMax", ((int) -> void), [JValue::Int(100)]).unwrap();
        let percent = (value.clamp(0.0, 1.0) * 100.0) as i32;
        jcall!(self.env, &seek_bar, "setProgress", ((int) -> void), [JValue::Int(percent)])
            .unwrap();

        // `WRAP_CONTENT` below lets the `SeekBar` shrink to its bare track
        // thickness (a few px) since it has no text to size itself against
        // like every other control here does - an explicit minimum height
        // keeps the track and thumb (and the touch target dragging it)
        // reasonably sized instead of hairline-thin.
        jcall!(self.env, &seek_bar, "setMinimumHeight", ((int) -> void), [JValue::Int(72)])
            .unwrap();

        set_layout_params!(self.env, &seek_bar, -1, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(seek_bar).unwrap()),
        }
    }

    fn create_spacer(&mut self, size: i32) -> Self::PlatformView {
        let view = new_widget!(self.env, "android/view/View", self.context);

        let scaled = (size * 3) as i32;
        set_layout_params!(self.env, &view, scaled, scaled);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(view).unwrap()),
        }
    }

    fn create_divider(&mut self) -> Self::PlatformView {
        let view = new_widget!(self.env, "android/view/View", self.context);

        jcall!(self.env, &view, "setBackgroundColor", ((int) -> void), [JValue::Int(0xFFC8C8C8u32 as i32)]).unwrap();

        set_layout_params!(self.env, &view, -1, 3);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(view).unwrap()),
        }
    }

    fn create_stack(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        let layout = new_widget!(self.env, "android/widget/LinearLayout", self.context);

        let orientation = match direction {
            LayoutDirection::Horizontal => 0,
            LayoutDirection::Vertical => 1,
        };
        jcall!(self.env, &layout, "setOrientation", ((int) -> void), [JValue::Int(orientation)])
            .unwrap();

        let real_spacing = (spacing * 3) as i32;

        for (idx, child_view) in children.into_iter().enumerate() {
            let local_child = self
                .env
                .new_local_ref(child_view.global_ref.as_obj())
                .unwrap();

            let layout_params = jnew!(self.env, "android/widget/LinearLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-2), JValue::Int(-2)]);

            self.env
                .set_field(&layout_params, "gravity", "I", JValue::Int(1))
                .unwrap();

            if spacing > 0 && idx > 0 {
                let (margin_l, margin_t) = if direction == LayoutDirection::Vertical {
                    (0, real_spacing)
                } else {
                    (real_spacing, 0)
                };

                jcall!(
                    self.env, &layout_params, "setMargins", ((int, int, int, int) -> void),
                    [JValue::Int(margin_l), JValue::Int(margin_t), JValue::Int(0), JValue::Int(0)]
                )
                .unwrap();
            }

            jcall!(
                self.env, &layout, "addView", (("android/view/View", "android/view/ViewGroup$LayoutParams") -> void),
                [JValue::Object(&local_child), JValue::Object(&layout_params)]
            ).unwrap();
        }

        jcall!(self.env, &layout, "requestLayout", (() -> void), []).unwrap();

        set_layout_params!(self.env, &layout, -1, -1);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(layout).unwrap()),
        }
    }

    fn create_scroll_view(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        let inner = new_widget!(self.env, "android/widget/LinearLayout", self.context);

        let orientation = match direction {
            LayoutDirection::Horizontal => 0,
            LayoutDirection::Vertical => 1,
        };
        jcall!(self.env, &inner, "setOrientation", ((int) -> void), [JValue::Int(orientation)])
            .unwrap();

        let real_spacing = spacing * 3;

        for (idx, child_view) in children.into_iter().enumerate() {
            let local_child = self
                .env
                .new_local_ref(child_view.global_ref.as_obj())
                .unwrap();

            let child_params = jnew!(self.env, "android/widget/LinearLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-2), JValue::Int(-2)]);
            self.env
                .set_field(&child_params, "gravity", "I", JValue::Int(1))
                .unwrap();

            if spacing > 0 && idx > 0 {
                let (margin_l, margin_t) = if direction == LayoutDirection::Vertical {
                    (0, real_spacing)
                } else {
                    (real_spacing, 0)
                };
                jcall!(self.env, &child_params, "setMargins", ((int, int, int, int) -> void), [JValue::Int(margin_l), JValue::Int(margin_t), JValue::Int(0), JValue::Int(0)]).unwrap();
            }

            jcall!(self.env, &inner, "addView", (("android/view/View", "android/view/ViewGroup$LayoutParams") -> void), [JValue::Object(&local_child), JValue::Object(&child_params)]).unwrap();
        }

        // Unlike `create_stack`'s own `(-1, -1)` (`MATCH_PARENT` both axes),
        // the scrollable axis must stay `WRAP_CONTENT` - a `MATCH_PARENT`
        // main axis would just clamp the content to the viewport's own
        // size, leaving nothing to scroll.
        let (inner_w, inner_h) = match direction {
            LayoutDirection::Horizontal => (-2, -1),
            LayoutDirection::Vertical => (-1, -2),
        };
        set_layout_params!(self.env, &inner, inner_w, inner_h);

        let scroll_class = match direction {
            LayoutDirection::Horizontal => "android/widget/HorizontalScrollView",
            LayoutDirection::Vertical => "android/widget/ScrollView",
        };
        let scroll_view = new_widget!(self.env, scroll_class, self.context);
        jcall!(self.env, &scroll_view, "addView", (("android/view/View") -> void), [JValue::Object(&inner)]).unwrap();

        set_layout_params!(self.env, &scroll_view, -1, -1);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(scroll_view).unwrap()),
        }
    }

    fn create_overlay(&mut self, children: Vec<Self::PlatformView>) -> Self::PlatformView {
        let frame = new_widget!(self.env, "android/widget/FrameLayout", self.context);

        for child_view in children {
            let local_child = self
                .env
                .new_local_ref(child_view.global_ref.as_obj())
                .unwrap();

            // Read back whatever `apply_offset_x`/`apply_offset_y` stashed
            // as a margin on the child's *current* `LayoutParams` (set by
            // its own `create_*`) before replacing it with a fresh
            // `FrameLayout$LayoutParams` - see those functions' doc
            // comments for why a margin is the carrier.
            let old_params = jcall!(self.env, &local_child, "getLayoutParams", (() -> "android/view/ViewGroup$LayoutParams"), []).ok().and_then(|r| r.l().ok());
            let (left, top) = match &old_params {
                Some(p)
                    if self
                        .env
                        .is_instance_of(p, "android/view/ViewGroup$MarginLayoutParams")
                        .unwrap_or(false) =>
                {
                    let l = self
                        .env
                        .get_field(p, "leftMargin", "I")
                        .ok()
                        .and_then(|v| v.i().ok())
                        .unwrap_or(0);
                    let t = self
                        .env
                        .get_field(p, "topMargin", "I")
                        .ok()
                        .and_then(|v| v.i().ok())
                        .unwrap_or(0);
                    (l, t)
                }
                _ => (0, 0),
            };

            let frame_params = jnew!(self.env, "android/widget/FrameLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-2), JValue::Int(-2)]);
            self.env
                .set_field(&frame_params, "leftMargin", "I", JValue::Int(left))
                .unwrap();
            self.env
                .set_field(&frame_params, "topMargin", "I", JValue::Int(top))
                .unwrap();
            jcall!(self.env, &local_child, "setLayoutParams", (("android/view/ViewGroup$LayoutParams") -> void), [JValue::Object(&frame_params)]).unwrap();

            // No `Axis::ZIndex` tracking on this backend (see
            // `build_style_registry`'s doc comment) - `addView` order is
            // the only stacking rule, later added draws on top, matching
            // the other two backends' tie-break default.
            jcall!(self.env, &frame, "addView", (("android/view/View") -> void), [JValue::Object(&local_child)]).unwrap();
        }

        set_layout_params!(self.env, &frame, -2, -2);

        AndroidView {
            global_ref: Arc::new(self.env.new_global_ref(frame).unwrap()),
        }
    }

    fn apply_style(&mut self, view: &Self::PlatformView, style: StyleProperty) {
        let local_view = self.env.new_local_ref(view.global_ref.as_obj()).unwrap();
        let StyleProperty(axis, value) = style;

        if axis == Axis::FontFamily {
            if let StyleValue::Asset(asset) = &value {
                apply_font_family(self.env, self.context, &local_view, asset);
            }
            return;
        }

        match STYLE_REGISTRY.get(&axis) {
            Some(applier) => applier(&mut *self.env, &local_view, &value),
            None => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "goyda(android): axis {axis:?} is not supported on this backend, skipped"
                );
            }
        }
    }
}
