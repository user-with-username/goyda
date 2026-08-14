//! Tests for `src/color.rs`.

use goyda_utils::Color;
use goyda_utils::color::{argb, css_rgba};

#[test]
fn argb_maps_named_colors_to_their_hex_value() {
    assert_eq!(argb(Color::PRIMARY), 0xFF6200EE);
    assert_eq!(argb(Color::GRAY), 0xFF888888);
    assert_eq!(argb(Color::GREEN), 0xFF4CAF50);
    assert_eq!(argb(Color::RED), 0xFFF44336);
    assert_eq!(argb(Color::BACKGROUND), 0xFFFFFFFF);
    assert_eq!(argb(Color::WHITE), 0xFFFFFFFF);
    assert_eq!(argb(Color::BLACK), 0xFF000000);
    assert_eq!(argb(Color::TRANSPARENT), 0x00000000);
    assert_eq!(argb(Color::BLUE), 0xFF2196F3);
    assert_eq!(argb(Color::ORANGE), 0xFFFF9800);
    assert_eq!(argb(Color::YELLOW), 0xFFFFEB3B);
    assert_eq!(argb(Color::PURPLE), 0xFF9C27B0);
}

#[test]
fn argb_custom_passes_the_hex_value_through_unchanged() {
    assert_eq!(argb(Color::Custom(0x11223344)), 0x11223344);
}

#[test]
fn css_rgba_decodes_argb_channels_and_normalizes_alpha() {
    assert_eq!(
        css_rgba(Color::Custom(0xFF112233)),
        "rgba(17, 34, 51, 1.000)"
    );
    assert_eq!(
        css_rgba(Color::Custom(0x00112233)),
        "rgba(17, 34, 51, 0.000)"
    );
    assert_eq!(
        css_rgba(Color::Custom(0x80112233)),
        "rgba(17, 34, 51, 0.502)"
    );
}

#[test]
fn css_rgba_of_transparent_is_fully_transparent() {
    assert_eq!(css_rgba(Color::TRANSPARENT), "rgba(0, 0, 0, 0.000)");
}
