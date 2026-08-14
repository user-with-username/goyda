/// A reference to a file under the project's `assets/` directory, such as an
/// image or a font.
///
/// ```
/// # use goyda_utils::Asset;
/// let logo = Asset::new("logo.png");
/// assert_eq!(logo.path(), "logo.png");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Asset {
    path: String,
    bytes: Option<&'static [u8]>,
}

impl Asset {
    /// Creates an asset reference from a path relative to `assets/`.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: normalize(path.into()), bytes: None }
    }

    /// Creates an asset whose file content is embedded in the binary.
    /// Typically produced by the `asset!` macro rather than called directly.
    pub fn embedded(path: impl Into<String>, bytes: &'static [u8]) -> Self {
        Self { path: normalize(path.into()), bytes: Some(bytes) }
    }

    /// The asset's path relative to the project's `assets/` directory.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The asset's embedded file content, if any.
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

/// The CSS `format()` hint for a font asset's `@font-face src`. `None` if
/// the asset's extension isn't a recognized font format.
pub fn font_format_hint(asset: &Asset) -> Option<&'static str> {
    match extension(asset)?.as_str() {
        "ttf" => Some("truetype"),
        "otf" => Some("opentype"),
        "woff" => Some("woff"),
        "woff2" => Some("woff2"),
        _ => None,
    }
}

/// The MIME type for an asset's content, based on its file extension. `None`
/// for unrecognized extensions.
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

/// Whether the asset's file extension is `.svg`.
pub fn is_svg(asset: &Asset) -> bool {
    extension(asset).as_deref() == Some("svg")
}
