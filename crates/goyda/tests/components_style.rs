//! Tests for `src/components/style.rs`: the `style_methods!` macro, as used
//! to generate `Component::p`/`px`/`py`/`m`/`mx`/`my`/`border_color`/
//! `border_width`/`rounded`/`shadow`/`opacity` in `components/mod.rs`.

use goyda::{Axis, Color, Component, Edge, StyleValue};

fn last_style(component: &Component) -> &goyda::StyleProperty {
    match component {
        Component::Styled { styles, .. } => styles.last().expect("no styles pushed"),
        _ => panic!("expected Component::Styled"),
    }
}

#[test]
fn spacing_methods_set_a_spacing_scale_index_not_a_raw_pixel_value() {
    let p = Component::text(|| "x".into()).p(3);
    assert!(matches!(last_style(&p).0, Axis::Padding(Edge::All)));
    assert!(matches!(last_style(&p).1, StyleValue::Spacing(3)));

    let px = Component::text(|| "x".into()).px(2);
    assert!(matches!(last_style(&px).0, Axis::Padding(Edge::Horizontal)));
    assert!(matches!(last_style(&px).1, StyleValue::Spacing(2)));

    let py = Component::text(|| "x".into()).py(2);
    assert!(matches!(last_style(&py).0, Axis::Padding(Edge::Vertical)));

    let m = Component::text(|| "x".into()).m(1);
    assert!(matches!(last_style(&m).0, Axis::Margin(Edge::All)));

    let mx = Component::text(|| "x".into()).mx(1);
    assert!(matches!(last_style(&mx).0, Axis::Margin(Edge::Horizontal)));

    let my = Component::text(|| "x".into()).my(1);
    assert!(matches!(last_style(&my).0, Axis::Margin(Edge::Vertical)));
}

#[test]
fn border_color_sets_color_value() {
    let c = Component::text(|| "x".into()).border_color(Color::GRAY);
    assert!(matches!(last_style(&c).0, Axis::BorderColor));
    assert!(matches!(last_style(&c).1, StyleValue::Color(Color::GRAY)));
}

#[test]
fn border_width_and_rounded_and_shadow_set_a_spacing_scale_index() {
    let bw = Component::text(|| "x".into()).border_width(1);
    assert!(matches!(last_style(&bw).0, Axis::BorderWidth));
    assert!(matches!(last_style(&bw).1, StyleValue::Spacing(1)));

    let r = Component::text(|| "x".into()).rounded(2);
    assert!(matches!(last_style(&r).0, Axis::BorderRadius));
    assert!(matches!(last_style(&r).1, StyleValue::Spacing(2)));

    let s = Component::text(|| "x".into()).shadow(4);
    assert!(matches!(last_style(&s).0, Axis::Shadow));
    assert!(matches!(last_style(&s).1, StyleValue::Spacing(4)));
}

#[test]
fn opacity_sets_a_raw_number_value() {
    let o = Component::text(|| "x".into()).opacity(0.5);
    assert!(matches!(last_style(&o).0, Axis::Opacity));
    assert!(matches!(last_style(&o).1, StyleValue::Number(v) if v == 0.5));
}
