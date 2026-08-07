use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::targets::{BuildContext, Platform, PlatformTarget};

pub struct IosTarget;

impl PlatformTarget for IosTarget {
    fn id(&self) -> Platform {
        Platform::Ios
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &["aarch64-apple-ios", "x86_64-apple-ios", "aarch64-apple-ios-sim"]
    }

    fn is_implemented(&self) -> bool {
        false
    }

    fn build(&self, _ctx: &BuildContext) -> Result<PathBuf> {
        bail!("iOS support is not implemented yet.")
    }
}
