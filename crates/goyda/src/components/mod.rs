pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod divider;
pub mod image;
pub mod layout;
pub mod link;
pub mod overlay;
pub mod progress;
pub mod radio_button;
pub mod scroll_view;
pub mod spacer;
pub mod style;
pub mod switch;
pub mod text;
pub mod text_input;
pub mod textarea;

pub use badge::Badge;
pub use button::Button;
pub use card::Card;
pub use checkbox::Checkbox;
pub use divider::Divider;
pub use goyda_utils::{Asset, Color};
pub use image::Image;
pub use layout::Stack;
pub use link::Link;
pub use overlay::Overlay;
pub use progress::Progress;
pub use radio_button::RadioButton;
pub use scroll_view::ScrollView;
pub use spacer::Spacer;
pub use style::{Align, Axis, Edge, StyleProperty, StyleValue};
pub use switch::Switch;
pub use text::Text;
pub use text_input::TextInput;
pub use textarea::Textarea;

use crate::core::Backend;
use crate::core::events::{Event, Update};
use crate::reactive::reactive;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct Modifier;

pub struct Handler {
    pub attach:
        fn(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event) + 'static>),
    pub callback: Rc<dyn Fn(Event) + 'static>,
}

pub enum Component {
    Text(Text),
    Button(Button),
    Image(Image),
    Stack(Stack),
    TextInput(TextInput),
    Checkbox(Checkbox),
    Switch(Switch),
    Progress(Progress),
    Spacer(Spacer),
    Divider(Divider),
    ScrollView(ScrollView),
    Textarea(Textarea),
    RadioButton(RadioButton),
    Overlay(Overlay),
    WithHandlers {
        component: Box<Component>,
        handlers: Vec<Handler>,
    },
    Styled {
        component: Box<Component>,
        styles: Vec<StyleProperty>,
    },
}

impl Component {
    pub fn text(compute: impl Fn() -> String + 'static) -> Self {
        Self::Text(Text {
            compute: Rc::new(compute),
        })
    }

    pub fn button(label: impl Into<String>) -> Self {
        Self::Button(Button { text: label.into() })
    }

    pub fn image(asset: impl Into<Asset>) -> Self {
        Self::Image(Image {
            asset: asset.into(),
        })
    }

    pub fn stack(direction: LayoutDirection, spacing: i32, children: Vec<Component>) -> Self {
        Self::Stack(Stack {
            direction,
            spacing,
            children,
        })
    }

    /// A stacking context (`position: absolute` children) - see
    /// [`Overlay::new`].
    pub fn overlay(children: Vec<Component>) -> Self {
        Self::Overlay(Overlay { children })
    }

    /// A scrollable [`Stack`] - give it a fixed size with `.height(px)`/
    /// `.width(px)` for scrolling to actually kick in.
    pub fn scroll_view(direction: LayoutDirection, spacing: i32, children: Vec<Component>) -> Self {
        Self::ScrollView(ScrollView {
            direction,
            spacing,
            children,
        })
    }

    /// A single-line editable text field.
    pub fn text_input(placeholder: impl Into<String>) -> Self {
        Self::TextInput(TextInput {
            placeholder: placeholder.into(),
            initial_text: String::new(),
        })
    }

    /// A multi-line editable text field.
    pub fn textarea(placeholder: impl Into<String>) -> Self {
        Self::Textarea(Textarea {
            placeholder: placeholder.into(),
            initial_text: String::new(),
        })
    }

    /// A labeled checkbox, starting `checked` or not.
    pub fn checkbox(label: impl Into<String>, checked: bool) -> Self {
        Self::Checkbox(Checkbox {
            label: label.into(),
            checked,
        })
    }

    /// A labeled radio button - selecting it deselects every other
    /// `RadioButton` sharing the same `group` string.
    pub fn radio_button(
        group: impl Into<String>,
        label: impl Into<String>,
        selected: bool,
    ) -> Self {
        Self::RadioButton(RadioButton {
            group: group.into(),
            label: label.into(),
            selected,
        })
    }

    /// An on/off toggle switch, starting `checked` or not.
    pub fn switch(checked: bool) -> Self {
        Self::Switch(Switch { checked })
    }

    /// A horizontal progress bar; `compute` should return values in `0.0..=1.0`.
    pub fn progress(compute: impl Fn() -> f32 + 'static) -> Self {
        Self::Progress(Progress {
            compute: Rc::new(compute),
        })
    }

    /// A fixed-size, invisible gap along the enclosing stack's axis.
    pub fn spacer(size: i32) -> Self {
        Self::Spacer(Spacer { size })
    }

    /// A thin line spanning the enclosing stack's cross axis.
    pub fn divider() -> Self {
        Self::Divider(Divider)
    }

    /// A pre-styled elevated surface - see [`Card::new`].
    pub fn card(children: Vec<Component>) -> Self {
        Card::new(children)
    }

    /// A small pill-shaped status label - see [`Badge::new`].
    pub fn badge(text: impl Into<String>, color: Color) -> Self {
        Badge::new(text, color)
    }

    /// Clickable, primary-colored text - see [`Link::new`].
    pub fn link(text: impl Into<String>, on_click: impl Fn() + 'static) -> Self {
        Link::new(text, on_click)
    }

    pub fn style(self, property: StyleProperty) -> Self {
        match self {
            Component::Styled {
                component,
                mut styles,
            } => {
                styles.push(property);
                Component::Styled { component, styles }
            }
            other => Component::Styled {
                component: Box::new(other),
                styles: vec![property],
            },
        }
    }

    pub fn color(self, color: Color) -> Self {
        self.style(StyleProperty(Axis::TextColor, StyleValue::Color(color)))
    }

    pub fn background(self, color: Color) -> Self {
        self.style(StyleProperty(
            Axis::BackgroundColor,
            StyleValue::Color(color),
        ))
    }

    pub fn font_size(self, size: i32) -> Self {
        self.style(StyleProperty(
            Axis::FontSize,
            StyleValue::Length(size as f32),
        ))
    }

    /// Sets a custom font, loaded from an asset (e.g. `.font("fonts/Inter-Bold.ttf")`).
    pub fn font(self, asset: impl Into<Asset>) -> Self {
        self.style(StyleProperty(
            Axis::FontFamily,
            StyleValue::Asset(asset.into()),
        ))
    }

    pub fn padding(self, horizontal: i32, vertical: i32) -> Self {
        self.style(StyleProperty(
            Axis::Padding(Edge::Horizontal),
            StyleValue::Length(horizontal as f32),
        ))
        .style(StyleProperty(
            Axis::Padding(Edge::Vertical),
            StyleValue::Length(vertical as f32),
        ))
    }

    /// A fixed width in pixels, overriding the component's own wrap-content
    /// sizing (and, on a `Stack`'s cross axis, [`Component::align`]'s
    /// `Stretch`).
    pub fn width(self, px: i32) -> Self {
        self.style(StyleProperty(Axis::Width, StyleValue::Length(px as f32)))
    }

    /// A fixed height in pixels - see [`Component::width`].
    pub fn height(self, px: i32) -> Self {
        self.style(StyleProperty(Axis::Height, StyleValue::Length(px as f32)))
    }

    /// Convenience for `.width(px).height(px)`.
    pub fn size(self, px: i32) -> Self {
        self.width(px).height(px)
    }

    pub fn text_align(self, align: Align) -> Self {
        self.style(StyleProperty(Axis::TextAlign, StyleValue::Align(align)))
    }

    pub fn bold(self) -> Self {
        self.style(StyleProperty(Axis::FontWeight, StyleValue::Bool(true)))
    }

    pub fn italic(self) -> Self {
        self.style(StyleProperty(Axis::FontStyle, StyleValue::Bool(true)))
    }

    /// Cross-axis alignment of a [`Stack`]'s children (defaults to
    /// `Stretch` if never set - each child fills the cross axis, same as
    /// every backend has always done).
    pub fn align(self, align: Align) -> Self {
        self.style(StyleProperty(Axis::AlignItems, StyleValue::Align(align)))
    }

    /// Main-axis distribution of a [`Stack`]'s children (defaults to
    /// `Start` - children packed together at the top/left with `spacing`
    /// between them, same as every backend has always done).
    pub fn justify(self, align: Align) -> Self {
        self.style(StyleProperty(
            Axis::JustifyContent,
            StyleValue::Align(align),
        ))
    }

    /// Distance between lines of wrapped/multi-line text, in pixels.
    pub fn line_height(self, px: i32) -> Self {
        self.style(StyleProperty(
            Axis::LineHeight,
            StyleValue::Length(px as f32),
        ))
    }

    /// Extra space between characters, in pixels.
    pub fn letter_spacing(self, px: i32) -> Self {
        self.style(StyleProperty(
            Axis::LetterSpacing,
            StyleValue::Length(px as f32),
        ))
    }

    pub fn underline(self) -> Self {
        self.style(StyleProperty(Axis::Underline, StyleValue::Bool(true)))
    }

    pub fn strikethrough(self) -> Self {
        self.style(StyleProperty(Axis::Strikethrough, StyleValue::Bool(true)))
    }

    /// Truncates to a single line with a trailing "…" instead of wrapping.
    pub fn ellipsis(self) -> Self {
        self.style(StyleProperty(
            Axis::TextOverflowEllipsis,
            StyleValue::Bool(true),
        ))
    }

    /// Clips content that overflows this component's bounds - mostly
    /// matters on the web backend (see [`Axis::Clip`]).
    pub fn clip(self) -> Self {
        self.style(StyleProperty(Axis::Clip, StyleValue::Bool(true)))
    }

    /// Extends `.shadow(px)` with a color (defaults to a neutral gray).
    pub fn shadow_color(self, color: Color) -> Self {
        self.style(StyleProperty(Axis::ShadowColor, StyleValue::Color(color)))
    }

    /// Positions a direct child of an [`Overlay`] `offset_x`/`offset_y`
    /// pixels from its top-left corner, out of normal flow - `position:
    /// absolute` in CSS terms. No-op outside an `Overlay`.
    pub fn offset(self, offset_x: i32, offset_y: i32) -> Self {
        self.style(StyleProperty(
            Axis::OffsetX,
            StyleValue::Length(offset_x as f32),
        ))
        .style(StyleProperty(
            Axis::OffsetY,
            StyleValue::Length(offset_y as f32),
        ))
    }

    /// Paint order among `Overlay` siblings (higher draws on top). No-op
    /// outside an `Overlay`.
    pub fn z_index(self, z: i32) -> Self {
        self.style(StyleProperty(Axis::ZIndex, StyleValue::Number(z as f32)))
    }

    fn with_handler(self, attach: RawAttach, callback: impl Fn(Event) + 'static) -> Self {
        match self {
            Component::WithHandlers {
                component,
                mut handlers,
            } => {
                handlers.push(Handler {
                    attach,
                    callback: Rc::new(callback),
                });
                Component::WithHandlers {
                    component,
                    handlers,
                }
            }
            other => Component::WithHandlers {
                component: Box::new(other),
                handlers: vec![Handler {
                    attach,
                    callback: Rc::new(callback),
                }],
            },
        }
    }

    /// Runs `action` on click. Equivalent to the `button { .. on_click: .. }`
    /// macro sugar, but works on any component (a [`Checkbox`] label, an
    /// [`Image`], a whole [`Stack`], ...), not just `Component::button`.
    pub fn on_click(self, action: impl Fn() + 'static) -> Self {
        self.with_handler(attach_click, move |_e| action())
    }

    /// Runs `action` on a press-and-hold (~500ms).
    pub fn on_long_click(self, action: impl Fn() + 'static) -> Self {
        self.with_handler(attach_long_click, move |_e| action())
    }

    /// Runs `handler` with the new state whenever a [`Checkbox`] or
    /// [`Switch`] is toggled.
    pub fn on_checked_change(self, handler: impl Fn(bool) + 'static) -> Self {
        self.with_handler(attach_checked_change, move |e| {
            if let Event::CheckedChanged(checked) = e {
                handler(checked);
            }
        })
    }

    /// Runs `handler` with a [`TextInput`]'s full current text on every
    /// keystroke - the ergonomic way to mirror what's typed into a
    /// [`Signal`](crate::reactive::Signal) and show it elsewhere:
    /// `Component::text_input("Name").on_text_changed(move |t| name.set(t))`.
    pub fn on_text_changed(self, handler: impl Fn(String) + 'static) -> Self {
        self.with_handler(attach_text_watcher, move |e| {
            if let Event::TextChanged { text, .. } = e {
                handler(text);
            }
        })
    }

    /// Runs `handler` with `true`/`false` when a component gains/loses
    /// keyboard focus (most useful on a [`TextInput`]).
    pub fn on_focus_change(self, handler: impl Fn(bool) + 'static) -> Self {
        self.with_handler(attach_focus_change, move |e| {
            if let Event::FocusChanged(focused) = e {
                handler(focused);
            }
        })
    }

    /// Runs `handler` with the new `0.0..=1.0` value whenever a [`Progress`]
    /// bar is clicked or dragged - it's a scrubber, not just a read-only
    /// indicator, so this is how an app finds out the user moved it (persist
    /// it back into whatever drives the bar's own `compute` closure, e.g.
    /// `.on_value_changed(move |v| loading = v)`).
    pub fn on_value_changed(self, handler: impl Fn(f32) + 'static) -> Self {
        self.with_handler(attach_seek, move |e| {
            if let Event::ValueChanged(value) = e {
                handler(value);
            }
        })
    }
}

type RawAttach = fn(*mut (), *const (), Rc<dyn Fn(Event)>);

fn attach_click(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_click::attach(backend, view, callback);
    }
}

fn attach_long_click(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_long_click::attach(backend, view, callback);
    }
}

fn attach_checked_change(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_checked_change::attach(backend, view, callback);
    }
}

fn attach_text_watcher(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_text_watcher::attach(backend, view, callback);
    }
}

fn attach_focus_change(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_focus_change::attach(backend, view, callback);
    }
}

fn attach_seek(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event)>) {
    unsafe {
        let backend = &mut *(backend_ptr as *mut crate::platform::ActiveBackend);
        let view = &*(view_ptr
            as *const <crate::platform::ActiveBackend as crate::core::Backend>::PlatformView);
        crate::platform::active_listeners::on_seek::attach(backend, view, callback);
    }
}

crate::style_methods! {
    p(usize) => Axis::Padding(Edge::All), StyleValue::Spacing;
    px(usize) => Axis::Padding(Edge::Horizontal), StyleValue::Spacing;
    py(usize) => Axis::Padding(Edge::Vertical), StyleValue::Spacing;
    m(usize) => Axis::Margin(Edge::All), StyleValue::Spacing;
    mx(usize) => Axis::Margin(Edge::Horizontal), StyleValue::Spacing;
    my(usize) => Axis::Margin(Edge::Vertical), StyleValue::Spacing;
    border_color(Color) => Axis::BorderColor, StyleValue::Color;
    border_width(usize) => Axis::BorderWidth, StyleValue::Spacing;
    rounded(usize) => Axis::BorderRadius, StyleValue::Spacing;
    shadow(usize) => Axis::Shadow, StyleValue::Spacing;
    opacity(f32) => Axis::Opacity, StyleValue::Number;
}

impl Component {
    pub fn render<B: Backend>(&self, backend: &mut B) -> B::PlatformView {
        match self {
            Component::Text(text_comp) => {
                let view = backend.create_text(&(text_comp.compute)());
                reactive(backend, &view, text_comp.compute.clone(), Update::SetText);
                view
            }
            Component::Button(btn_comp) => backend.create_button(&btn_comp.text),
            Component::Image(img_comp) => backend.create_image(&img_comp.asset),
            Component::Stack(stack_comp) => {
                let views = stack_comp
                    .children
                    .iter()
                    .map(|c| c.render(backend))
                    .collect();
                backend.create_stack(stack_comp.direction, stack_comp.spacing, views)
            }
            Component::TextInput(input_comp) => {
                backend.create_text_input(&input_comp.placeholder, &input_comp.initial_text)
            }
            Component::Checkbox(checkbox_comp) => {
                backend.create_checkbox(&checkbox_comp.label, checkbox_comp.checked)
            }
            Component::Switch(switch_comp) => backend.create_switch(switch_comp.checked),
            Component::Progress(progress_comp) => {
                let view = backend.create_progress((progress_comp.compute)());
                reactive(
                    backend,
                    &view,
                    progress_comp.compute.clone(),
                    Update::SetProgress,
                );
                view
            }
            Component::Spacer(spacer_comp) => backend.create_spacer(spacer_comp.size),
            Component::Divider(_) => backend.create_divider(),
            Component::ScrollView(scroll_comp) => {
                let views = scroll_comp
                    .children
                    .iter()
                    .map(|c| c.render(backend))
                    .collect();
                backend.create_scroll_view(scroll_comp.direction, scroll_comp.spacing, views)
            }
            Component::Textarea(textarea_comp) => {
                backend.create_textarea(&textarea_comp.placeholder, &textarea_comp.initial_text)
            }
            Component::RadioButton(radio_comp) => backend.create_radio_button(
                &radio_comp.group,
                &radio_comp.label,
                radio_comp.selected,
            ),
            Component::Overlay(overlay_comp) => {
                let views = overlay_comp
                    .children
                    .iter()
                    .map(|c| c.render(backend))
                    .collect();
                backend.create_overlay(views)
            }
            Component::WithHandlers {
                component,
                handlers,
            } => {
                let view = component.render(backend);
                for handler in handlers {
                    (handler.attach)(
                        backend as *mut B as *mut (),
                        &view as *const B::PlatformView as *const (),
                        handler.callback.clone(),
                    );
                }
                view
            }
            Component::Styled { component, styles } => {
                let view = component.render(backend);
                for style in styles {
                    backend.apply_style(&view, style.clone());
                }
                view
            }
        }
    }
}
