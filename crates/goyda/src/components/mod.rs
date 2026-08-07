pub mod text;
pub mod button;
pub mod layout;
pub mod style;

pub use text::Text;
pub use button::Button;
pub use layout::Stack;
pub use style::{Axis, Edge, StyleProperty, StyleValue};

use crate::core::Backend;
use crate::core::events::{Event, Update};
use crate::reactive::reactive;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDirection { Horizontal, Vertical }

#[derive(Debug, Clone, Copy)]
pub enum Color {
    PRIMARY,
    GRAY,
    GREEN,
    RED,
    BACKGROUND,
    Custom(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct Modifier;

pub struct Handler {
    pub attach: fn(backend_ptr: *mut (), view_ptr: *const (), callback: Rc<dyn Fn(Event) + 'static>),
    pub callback: Rc<dyn Fn(Event) + 'static>,
}

pub enum Component {
    Text(Text),
    Button(Button),
    Stack(Stack),
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
        Self::Text(Text { compute: Rc::new(compute) })
    }
    
    pub fn button(label: impl Into<String>) -> Self {
        Self::Button(Button { text: label.into() })
    }
    
    pub fn stack(direction: LayoutDirection, spacing: i32, children: Vec<Component>) -> Self {
        Self::Stack(Stack { direction, spacing, children })
    }

    pub fn style(self, property: StyleProperty) -> Self {
        match self {
            Component::Styled { component, mut styles } => {
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
        self.style(StyleProperty(Axis::BackgroundColor, StyleValue::Color(color)))
    }

    pub fn font_size(self, size: i32) -> Self {
        self.style(StyleProperty(Axis::FontSize, StyleValue::Length(size as f32)))
    }

    pub fn padding(self, horizontal: i32, vertical: i32) -> Self {
        self
            .style(StyleProperty(Axis::Padding(Edge::Horizontal), StyleValue::Length(horizontal as f32)))
            .style(StyleProperty(Axis::Padding(Edge::Vertical), StyleValue::Length(vertical as f32)))
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
            Component::Button(btn_comp) => {
                backend.create_button(&btn_comp.text)
            }
            Component::Stack(stack_comp) => {
                let views = stack_comp.children.iter().map(|c| c.render(backend)).collect();
                backend.create_stack(stack_comp.direction, stack_comp.spacing, views)
            }
            Component::WithHandlers { component, handlers } => {
                let view = component.render(backend);
                for handler in handlers {
                    (handler.attach)(
                        backend as *mut B as *mut (),
                        &view as *const B::PlatformView as *const (),
                        handler.callback.clone()
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
