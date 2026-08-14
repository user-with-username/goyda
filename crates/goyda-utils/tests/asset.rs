//! Tests for `src/asset.rs`.

use goyda_utils::Asset;
use goyda_utils::asset::{extension, font_format_hint, mime_type, is_svg};

#[test]
fn new_normalizes_a_leading_slash() {
    assert_eq!(Asset::new("/logo.svg").path(), "logo.svg");
}

#[test]
fn new_normalizes_backslashes_to_forward_slashes() {
    assert_eq!(Asset::new("icons\\star.png").path(), "icons/star.png");
}

#[test]
fn new_has_no_embedded_bytes() {
    assert_eq!(Asset::new("a.png").bytes(), None);
}

#[test]
fn embedded_keeps_its_bytes() {
    let a = Asset::embedded("a.png", b"data");
    assert_eq!(a.bytes(), Some(&b"data"[..]));
    assert_eq!(a.path(), "a.png");
}

#[test]
fn from_str_and_from_string_construct_a_plain_asset() {
    let from_str: Asset = "x.png".into();
    assert_eq!(from_str, Asset::new("x.png"));

    let from_string: Asset = String::from("y.png").into();
    assert_eq!(from_string, Asset::new("y.png"));
}

#[test]
fn assets_with_the_same_normalized_path_are_equal() {
    assert_eq!(Asset::new("/a/b.png"), Asset::new("a/b.png"));
}

#[test]
fn extension_is_lowercased_and_absent_for_extensionless_paths() {
    assert_eq!(extension(&Asset::new("Logo.SVG")), Some("svg".to_string()));
    assert_eq!(extension(&Asset::new("README")), None);
}

#[test]
fn font_format_hint_covers_known_font_extensions_only() {
    assert_eq!(font_format_hint(&Asset::new("a.ttf")), Some("truetype"));
    assert_eq!(font_format_hint(&Asset::new("a.otf")), Some("opentype"));
    assert_eq!(font_format_hint(&Asset::new("a.woff")), Some("woff"));
    assert_eq!(font_format_hint(&Asset::new("a.woff2")), Some("woff2"));
    assert_eq!(font_format_hint(&Asset::new("a.png")), None);
}

#[test]
fn mime_type_covers_known_image_and_font_extensions() {
    assert_eq!(mime_type(&Asset::new("a.svg")), Some("image/svg+xml"));
    assert_eq!(mime_type(&Asset::new("a.png")), Some("image/png"));
    assert_eq!(mime_type(&Asset::new("a.jpg")), Some("image/jpeg"));
    assert_eq!(mime_type(&Asset::new("a.jpeg")), Some("image/jpeg"));
    assert_eq!(mime_type(&Asset::new("a.gif")), Some("image/gif"));
    assert_eq!(mime_type(&Asset::new("a.webp")), Some("image/webp"));
    assert_eq!(mime_type(&Asset::new("a.ttf")), Some("font/ttf"));
    assert_eq!(mime_type(&Asset::new("a.otf")), Some("font/otf"));
    assert_eq!(mime_type(&Asset::new("a.woff")), Some("font/woff"));
    assert_eq!(mime_type(&Asset::new("a.woff2")), Some("font/woff2"));
    assert_eq!(mime_type(&Asset::new("a.bin")), None);
}

#[test]
fn is_svg_checks_extension_only() {
    assert!(is_svg(&Asset::new("logo.svg")));
    assert!(is_svg(&Asset::new("logo.SVG")));
    assert!(!is_svg(&Asset::new("logo.png")));
    assert!(!is_svg(&Asset::new("logo")));
}
