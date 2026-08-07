use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::targets::{BuildContext, Platform, PlatformTarget};

pub struct WindowsTarget;

impl PlatformTarget for WindowsTarget {
    fn id(&self) -> Platform {
        Platform::Windows
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &[
            "x86_64-pc-windows-msvc",
            "i686-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
        ]
    }

    fn is_implemented(&self) -> bool {
        false
    }

    fn build(&self, _ctx: &BuildContext) -> Result<PathBuf> {
        bail!("Windows support is not implemented yet.")
    }
}
