//! Tests for `src/components/image.rs`.

use goyda::Component;
use goyda::components::{Asset, Image};

#[test]
fn image_new_wraps_asset() {
    match Image::new("logo.svg") {
        Component::Image(img) => assert_eq!(img.asset, Asset::new("logo.svg")),
        _ => panic!("expected Component::Image"),
    }
}

#[test]
fn component_image_accepts_asset_conversion() {
    match Component::image("icons/star.png") {
        Component::Image(img) => assert_eq!(img.asset.path(), "icons/star.png"),
        _ => panic!("expected Component::Image"),
    }
}
