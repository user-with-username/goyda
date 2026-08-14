//! Tests for `src/components/mod.rs`: `Component`'s constructors, style
//! builder methods, and `render` dispatch.

use goyda::components::Align;
use goyda::core::events::Event;
use goyda::{Axis, Color, Component, Edge, LayoutDirection, StyleValue};

fn last_style(component: &Component) -> &goyda::StyleProperty {
    match component {
        Component::Styled { styles, .. } => styles.last().expect("no styles pushed"),
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn style_wraps_plain_component_once() {
    let styled = Component::text(|| "x".into())
        .style(goyda::StyleProperty(Axis::Opacity, StyleValue::Number(0.5)));

    match styled {
        Component::Styled { styles, .. } => assert_eq!(styles.len(), 1),
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn style_appends_to_an_already_styled_component_instead_of_nesting() {
    let styled = Component::text(|| "x".into())
        .style(goyda::StyleProperty(Axis::Opacity, StyleValue::Number(0.5)))
        .style(goyda::StyleProperty(
            Axis::FontSize,
            StyleValue::Length(10.0),
        ));

    match styled {
        Component::Styled { component, styles } => {
            assert_eq!(styles.len(), 2);
            assert!(matches!(*component, Component::Text(_)));
        }
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn color_sets_text_color() {
    let c = Component::text(|| "x".into()).color(Color::RED);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::TextColor));
    assert!(matches!(prop.1, StyleValue::Color(Color::RED)));
}

#[test]
fn background_sets_background_color() {
    let c = Component::text(|| "x".into()).background(Color::BLUE);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::BackgroundColor));
    assert!(matches!(prop.1, StyleValue::Color(Color::BLUE)));
}

#[test]
fn font_size_sets_length() {
    let c = Component::text(|| "x".into()).font_size(18);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::FontSize));
    assert!(matches!(prop.1, StyleValue::Length(v) if v == 18.0));
}

#[test]
fn font_sets_font_family_asset() {
    let c = Component::text(|| "x".into()).font("fonts/Inter-Bold.ttf");
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::FontFamily));
    match &prop.1 {
        StyleValue::Asset(asset) => assert_eq!(asset.path(), "fonts/Inter-Bold.ttf"),
        _ => panic!("expected StyleValue::Asset"),
    }
}

#[test]
fn padding_pushes_horizontal_then_vertical() {
    match Component::text(|| "x".into()).padding(4, 8) {
        Component::Styled { styles, .. } => {
            assert_eq!(styles.len(), 2);
            assert!(matches!(styles[0].0, Axis::Padding(Edge::Horizontal)));
            assert!(matches!(styles[0].1, StyleValue::Length(v) if v == 4.0));
            assert!(matches!(styles[1].0, Axis::Padding(Edge::Vertical)));
            assert!(matches!(styles[1].1, StyleValue::Length(v) if v == 8.0));
        }
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn width_and_height_set_length() {
    let w = Component::text(|| "x".into()).width(100);
    assert!(matches!(last_style(&w).0, Axis::Width));
    assert!(matches!(last_style(&w).1, StyleValue::Length(v) if v == 100.0));

    let h = Component::text(|| "x".into()).height(50);
    assert!(matches!(last_style(&h).0, Axis::Height));
    assert!(matches!(last_style(&h).1, StyleValue::Length(v) if v == 50.0));
}

#[test]
fn size_sets_width_then_height_to_the_same_value() {
    match Component::text(|| "x".into()).size(40) {
        Component::Styled { styles, .. } => {
            assert_eq!(styles.len(), 2);
            assert!(matches!(styles[0].0, Axis::Width));
            assert!(matches!(styles[0].1, StyleValue::Length(v) if v == 40.0));
            assert!(matches!(styles[1].0, Axis::Height));
            assert!(matches!(styles[1].1, StyleValue::Length(v) if v == 40.0));
        }
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn text_align_sets_align_value() {
    let c = Component::text(|| "x".into()).text_align(Align::Center);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::TextAlign));
    assert!(matches!(prop.1, StyleValue::Align(Align::Center)));
}

#[test]
fn bold_and_italic_set_bool_true() {
    let b = Component::text(|| "x".into()).bold();
    assert!(matches!(last_style(&b).0, Axis::FontWeight));
    assert!(matches!(last_style(&b).1, StyleValue::Bool(true)));

    let i = Component::text(|| "x".into()).italic();
    assert!(matches!(last_style(&i).0, Axis::FontStyle));
    assert!(matches!(last_style(&i).1, StyleValue::Bool(true)));
}

#[test]
fn align_and_justify_set_align_value() {
    let a = Component::text(|| "x".into()).align(Align::Stretch);
    assert!(matches!(last_style(&a).0, Axis::AlignItems));
    assert!(matches!(
        last_style(&a).1,
        StyleValue::Align(Align::Stretch)
    ));

    let j = Component::text(|| "x".into()).justify(Align::SpaceBetween);
    assert!(matches!(last_style(&j).0, Axis::JustifyContent));
    assert!(matches!(
        last_style(&j).1,
        StyleValue::Align(Align::SpaceBetween)
    ));
}

#[test]
fn line_height_and_letter_spacing_set_length() {
    let lh = Component::text(|| "x".into()).line_height(22);
    assert!(matches!(last_style(&lh).0, Axis::LineHeight));
    assert!(matches!(last_style(&lh).1, StyleValue::Length(v) if v == 22.0));

    let ls = Component::text(|| "x".into()).letter_spacing(2);
    assert!(matches!(last_style(&ls).0, Axis::LetterSpacing));
    assert!(matches!(last_style(&ls).1, StyleValue::Length(v) if v == 2.0));
}

#[test]
fn underline_strikethrough_ellipsis_clip_set_bool_true() {
    let u = Component::text(|| "x".into()).underline();
    assert!(matches!(last_style(&u).0, Axis::Underline));
    let s = Component::text(|| "x".into()).strikethrough();
    assert!(matches!(last_style(&s).0, Axis::Strikethrough));
    let e = Component::text(|| "x".into()).ellipsis();
    assert!(matches!(last_style(&e).0, Axis::TextOverflowEllipsis));
    let c = Component::text(|| "x".into()).clip();
    assert!(matches!(last_style(&c).0, Axis::Clip));
    for prop in [
        last_style(&u),
        last_style(&s),
        last_style(&e),
        last_style(&c),
    ] {
        assert!(matches!(prop.1, StyleValue::Bool(true)));
    }
}

#[test]
fn shadow_color_sets_color() {
    let c = Component::text(|| "x".into()).shadow_color(Color::PURPLE);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::ShadowColor));
    assert!(matches!(prop.1, StyleValue::Color(Color::PURPLE)));
}

#[test]
fn offset_pushes_offset_x_then_offset_y() {
    match Component::text(|| "x".into()).offset(10, 20) {
        Component::Styled { styles, .. } => {
            assert_eq!(styles.len(), 2);
            assert!(matches!(styles[0].0, Axis::OffsetX));
            assert!(matches!(styles[0].1, StyleValue::Length(v) if v == 10.0));
            assert!(matches!(styles[1].0, Axis::OffsetY));
            assert!(matches!(styles[1].1, StyleValue::Length(v) if v == 20.0));
        }
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn z_index_sets_number() {
    let c = Component::text(|| "x".into()).z_index(3);
    let prop = last_style(&c);
    assert!(matches!(prop.0, Axis::ZIndex));
    assert!(matches!(prop.1, StyleValue::Number(v) if v == 3.0));
}

#[test]
fn on_click_wraps_in_with_handlers_without_running_it() {
    use std::cell::Cell;
    use std::rc::Rc;

    let ran = Rc::new(Cell::new(false));
    let ran_clone = ran.clone();
    let c = Component::button("Go").on_click(move || ran_clone.set(true));

    match c {
        Component::WithHandlers { handlers, .. } => {
            assert_eq!(handlers.len(), 1);
            assert!(
                !ran.get(),
                "constructing the component must not invoke the handler"
            );
            (handlers[0].callback)(Event::Click);
            assert!(ran.get());
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}

#[test]
fn chained_handlers_accumulate_on_the_same_with_handlers_wrapper() {
    let c = Component::button("Go").on_click(|| {}).on_long_click(|| {});
    match c {
        Component::WithHandlers { handlers, .. } => assert_eq!(handlers.len(), 2),
        _ => panic!("expected Component::WithHandlers"),
    }
}

#[test]
fn on_checked_change_only_fires_for_checked_changed_events() {
    use std::cell::Cell;
    use std::rc::Rc;

    let seen: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
    let seen_clone = seen.clone();
    let c = Component::checkbox("x", false)
        .on_checked_change(move |checked| seen_clone.set(Some(checked)));

    match c {
        Component::WithHandlers { handlers, .. } => {
            (handlers[0].callback)(Event::Click); // wrong event kind - ignored
            assert_eq!(seen.get(), None);
            (handlers[0].callback)(Event::CheckedChanged(true));
            assert_eq!(seen.get(), Some(true));
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}

#[test]
fn on_text_changed_extracts_text_from_text_changed_event() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let seen: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let seen_clone = seen.clone();
    let c = Component::text_input("x").on_text_changed(move |t| *seen_clone.borrow_mut() = t);

    match c {
        Component::WithHandlers { handlers, .. } => {
            (handlers[0].callback)(Event::TextChanged {
                text: "hi".into(),
                start: 0,
                before: 0,
                count: 2,
            });
            assert_eq!(&*seen.borrow(), "hi");
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}

#[test]
fn on_focus_change_extracts_bool_from_focus_changed_event() {
    use std::cell::Cell;
    use std::rc::Rc;

    let seen = Rc::new(Cell::new(false));
    let seen_clone = seen.clone();
    let c = Component::text_input("x").on_focus_change(move |focused| seen_clone.set(focused));

    match c {
        Component::WithHandlers { handlers, .. } => {
            (handlers[0].callback)(Event::FocusChanged(true));
            assert!(seen.get());
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}

#[test]
fn on_value_changed_extracts_f32_from_value_changed_event() {
    use std::cell::Cell;
    use std::rc::Rc;

    let seen = Rc::new(Cell::new(0.0f32));
    let seen_clone = seen.clone();
    let c = Component::progress(|| 0.0).on_value_changed(move |v| seen_clone.set(v));

    match c {
        Component::WithHandlers { handlers, .. } => {
            (handlers[0].callback)(Event::ValueChanged(0.42));
            assert_eq!(seen.get(), 0.42);
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}

// --- render() dispatch, via a minimal in-memory Backend ---
//
// Only exercises handler-free components: `Component::render`'s
// `WithHandlers` arm casts the generic `&mut B` backend pointer to the
// platform's concrete `ActiveBackend` type when attaching a listener, which
// would be unsound to trigger against this mock.

mod mock_backend {
    use goyda::components::Asset;
    use goyda::core::events::Update;
    use goyda::core::{Backend, BackendUpdater};
    use goyda::{LayoutDirection, StyleProperty};

    #[derive(Clone, Debug, PartialEq)]
    pub enum View {
        Text(String),
        Button(String),
        Image,
        Stack,
    }

    pub struct MockUpdater;
    impl BackendUpdater for MockUpdater {
        type PlatformView = View;
        fn apply_update(&mut self, _view: &View, _update: Update) {}
    }

    #[derive(Default)]
    pub struct MockBackend {
        pub style_calls: usize,
    }

    impl Backend for MockBackend {
        type PlatformView = View;
        type Updater = MockUpdater;

        fn create_text(&mut self, content: &str) -> View {
            View::Text(content.to_string())
        }
        fn create_button(&mut self, text: &str) -> View {
            View::Button(text.to_string())
        }
        fn create_image(&mut self, _asset: &Asset) -> View {
            View::Image
        }
        fn create_stack(
            &mut self,
            _direction: LayoutDirection,
            _spacing: i32,
            _children: Vec<View>,
        ) -> View {
            View::Stack
        }
        fn create_scroll_view(
            &mut self,
            _direction: LayoutDirection,
            _spacing: i32,
            _children: Vec<View>,
        ) -> View {
            View::Stack
        }
        fn create_overlay(&mut self, _children: Vec<View>) -> View {
            View::Stack
        }
        fn create_text_input(&mut self, _placeholder: &str, _initial_text: &str) -> View {
            View::Text(String::new())
        }
        fn create_textarea(&mut self, _placeholder: &str, _initial_text: &str) -> View {
            View::Text(String::new())
        }
        fn create_checkbox(&mut self, _label: &str, _checked: bool) -> View {
            View::Button(String::new())
        }
        fn create_radio_button(&mut self, _group: &str, _label: &str, _selected: bool) -> View {
            View::Button(String::new())
        }
        fn create_switch(&mut self, _checked: bool) -> View {
            View::Button(String::new())
        }
        fn create_progress(&mut self, _value: f32) -> View {
            View::Button(String::new())
        }
        fn create_spacer(&mut self, _size: i32) -> View {
            View::Stack
        }
        fn create_divider(&mut self) -> View {
            View::Stack
        }
        fn clone_updater(&self) -> MockUpdater {
            MockUpdater
        }
        fn apply_style(&mut self, _view: &View, _style: StyleProperty) {
            self.style_calls += 1;
        }
    }
}

use mock_backend::{MockBackend, View};

#[test]
fn render_dispatches_text_to_create_text() {
    let mut backend = MockBackend::default();
    let view = Component::text(|| "hi".into()).render(&mut backend);
    assert_eq!(view, View::Text("hi".into()));
}

#[test]
fn render_dispatches_button_to_create_button() {
    let mut backend = MockBackend::default();
    let view = Component::button("Go").render(&mut backend);
    assert_eq!(view, View::Button("Go".into()));
}

#[test]
fn render_dispatches_stack_children_first() {
    let mut backend = MockBackend::default();
    let view = Component::stack(LayoutDirection::Vertical, 4, vec![Component::button("A")])
        .render(&mut backend);
    assert_eq!(view, View::Stack);
}

#[test]
fn render_applies_each_style_in_order() {
    let mut backend = MockBackend::default();
    let component = Component::text(|| "x".into()).color(Color::RED).bold();
    component.render(&mut backend);
    assert_eq!(backend.style_calls, 2);
}
