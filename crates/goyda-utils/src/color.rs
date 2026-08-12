/// Semantic color palette shared by every backend. `Custom` carries a raw
/// `0xAARRGGBB` value for anything outside the named set.
#[derive(Debug, Clone, Copy)]
pub enum Color {
    PRIMARY,
    GRAY,
    GREEN,
    RED,
    BACKGROUND,
    Custom(u32),
}

/// Resolves a [`Color`] to its canonical `0xAARRGGBB` value - the single
/// source of truth every backend's platform-specific color representation
/// (Android's signed 32-bit `int`, the web's `rgba(...)` string, ...) is
/// derived from.
pub fn argb(color: Color) -> u32 {
    match color {
        Color::PRIMARY => 0xFF6200EE,
        Color::GRAY => 0xFF888888,
        Color::GREEN => 0xFF4CAF50,
        Color::RED => 0xFFF44336,
        Color::BACKGROUND => 0xFFFFFFFF,
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
