/// A reference to a file the project ships under its `assets/` directory
/// (images, fonts, ...). `path` is relative to that directory, e.g.
/// `"logo.png"` or `"fonts/Inter-Bold.ttf"` - a leading slash is stripped
/// and backslashes are normalized, since every backend resolves it relative
/// to wherever it packages or serves assets from (the APK's `assets/` zip
/// entry, the web build's `assets/` directory, ...).
///
/// An `Asset` can either carry its file's bytes inline (`bytes` is `Some`,
/// produced by the `asset!` macro via `include_bytes!`) or defer to `path`
/// alone (produced by `asset_ref!` or a plain string, resolved by each
/// backend at runtime from wherever it packages/serves assets). Backends
/// prefer `bytes` when present since it needs no platform-specific
/// filesystem/network access at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Asset {
    path: String,
    bytes: Option<&'static [u8]>,
}

impl Asset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: normalize(path.into()), bytes: None }
    }

    /// Builds an asset whose content is already embedded in the binary.
    /// Used by the `asset!` macro - not usually called directly.
    pub fn embedded(path: impl Into<String>, bytes: &'static [u8]) -> Self {
        Self { path: normalize(path.into()), bytes: Some(bytes) }
    }

    /// The asset's path relative to the project's `assets/` directory.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The asset's embedded content, if `asset!` produced it. `None` for
    /// path-only assets (`asset_ref!`, or a plain string), which backends
    /// resolve at runtime instead.
    pub fn bytes(&self) -> Option<&'static [u8]> {
        self.bytes
    }
}

fn normalize(path: String) -> String {
    path.trim_start_matches('/').replace('\\', "/")
}

impl From<&str> for Asset {
    fn from(value: &str) -> Self {
        Asset::new(value)
    }
}

impl From<String> for Asset {
    fn from(value: String) -> Self {
        Asset::new(value)
    }
}

/// The asset's lowercased file extension, if it has one.
pub fn extension(asset: &Asset) -> Option<String> {
    std::path::Path::new(asset.path())
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// The CSS `format()` hint for a font asset's `@font-face src`, based on its
/// file extension. `None` for unrecognized extensions - the browser will
/// still try to sniff the format itself.
pub fn font_format_hint(asset: &Asset) -> Option<&'static str> {
    match extension(asset)?.as_str() {
        "ttf" => Some("truetype"),
        "otf" => Some("opentype"),
        "woff" => Some("woff"),
        "woff2" => Some("woff2"),
        _ => None,
    }
}

/// The MIME type for an asset's content, based on its file extension. Needed
/// wherever an asset's bytes are handed to a platform API that doesn't sniff
/// the format itself (e.g. constructing a web `Blob`) - without it, an SVG's
/// bytes are indistinguishable from arbitrary binary data and won't render.
pub fn mime_type(asset: &Asset) -> Option<&'static str> {
    match extension(asset)?.as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        _ => None,
    }
}

/// Whether the asset's file extension is `.svg` - the one image format that
/// platform raster decoders (Android's `BitmapFactory`) can't handle
/// natively and needs rasterizing first.
pub fn is_svg(asset: &Asset) -> bool {
    extension(asset).as_deref() == Some("svg")
}
