use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::targets::{BuildContext, Platform, PlatformTarget};

pub struct WebTarget;

impl PlatformTarget for WebTarget {
    fn id(&self) -> Platform {
        Platform::Web
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &["wasm32-unknown-unknown"]
    }

    fn is_implemented(&self) -> bool {
        false
    }

    fn build(&self, _ctx: &BuildContext) -> Result<PathBuf> {
        bail!("Web support is not implemented yet.")
    }
}
