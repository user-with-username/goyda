/// A named color from the theme's palette, or a custom `0xAARRGGBB` value.
///
/// ```
/// # use goyda_utils::Color;
/// let c = Color::PRIMARY;
/// let custom = Color::Custom(0xFF112233);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Color {
    PRIMARY,
    GRAY,
    GREEN,
    RED,
    BACKGROUND,
    WHITE,
    BLACK,
    TRANSPARENT,
    BLUE,
    ORANGE,
    YELLOW,
    PURPLE,
    Custom(u32),
}

/// Resolves a [`Color`] to its `0xAARRGGBB` value.
pub fn argb(color: Color) -> u32 {
    match color {
        Color::PRIMARY => 0xFF6200EE,
        Color::GRAY => 0xFF888888,
        Color::GREEN => 0xFF4CAF50,
        Color::RED => 0xFFF44336,
        Color::BACKGROUND => 0xFFFFFFFF,
        Color::WHITE => 0xFFFFFFFF,
        Color::BLACK => 0xFF000000,
        Color::TRANSPARENT => 0x00000000,
        Color::BLUE => 0xFF2196F3,
        Color::ORANGE => 0xFFFF9800,
        Color::YELLOW => 0xFFFFEB3B,
        Color::PURPLE => 0xFF9C27B0,
        Color::Custom(hex) => hex,
    }
}

/// Renders a [`Color`] as a CSS `rgba(r, g, b, a)` string.
pub fn css_rgba(color: Color) -> String {
    let value = argb(color);
    let a = ((value >> 24) & 0xFF) as f32 / 255.0;
    let r = (value >> 16) & 0xFF;
    let g = (value >> 8) & 0xFF;
    let b = value & 0xFF;

    format!("rgba({r}, {g}, {b}, {a:.3})")
}
