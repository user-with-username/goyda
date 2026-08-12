use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod android;
pub mod ios;
pub mod web;
pub mod windows;

pub struct BuildContext {
    pub manifest_dir: PathBuf,
    pub target_triple: String,
    pub crate_name: String,
    pub lib_name: String,
    pub target_directory: PathBuf,
    pub start: std::time::Instant,
}

pub trait PlatformTarget {
    fn id(&self) -> Platform;
    fn supported_triples(&self) -> &'static [&'static str];

    fn is_implemented(&self) -> bool {
        true
    }

    fn validate_triple(&self, triple: &str) -> Result<()> {
        if self.supported_triples().contains(&triple) {
            Ok(())
        } else {
            bail!(
                "Target triple '{}' is not supported for platform '{}'. Supported triples: {:?}",
                triple,
                self.id().as_str(),
                self.supported_triples()
            )
        }
    }

    fn build(&self, ctx: &BuildContext) -> Result<PathBuf>;

    fn run(&self, _ctx: &BuildContext, _artifact_path: &Path) -> Result<()> {
        bail!(
            "Running on device/emulator is not yet implemented for platform '{}'",
            self.id().as_str()
        );
    }

    fn stream_logs(&self, _ctx: &BuildContext, _start: std::time::Instant) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Android,
    Ios,
    Web,
    Windows,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Ios => "ios",
            Platform::Web => "web",
            Platform::Windows => "windows",
        }
    }

    pub fn handler(&self) -> Box<dyn PlatformTarget> {
        match self {
            Platform::Android => Box::new(android::AndroidTarget),
            Platform::Ios => Box::new(ios::IosTarget),
            Platform::Web => Box::new(web::WebTarget),
            Platform::Windows => Box::new(windows::WindowsTarget),
        }
    }

    pub fn all() -> &'static [Platform] {
        &[Platform::Android, Platform::Ios, Platform::Web, Platform::Windows]
    }
}

impl FromStr for Platform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "android" => Ok(Platform::Android),
            "ios" => Ok(Platform::Ios),
            "web" => Ok(Platform::Web),
            "windows" => Ok(Platform::Windows),
            other => bail!(
                "Unknown platform '{}'. Available platforms: {}",
                other,
                Platform::all()
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}