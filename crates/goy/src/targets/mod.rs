use anyhow::{Result, bail};
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
    /// `--release` on `goy compile`/`goy run` - each platform's `build`
    /// threads this into its own `cargo build` (`--release` flag, `release`
    /// vs `debug` output subdir); `compile` additionally copies the
    /// finished artifact into `<manifest_dir>/release/<platform>/` when
    /// this is set, a stable, obvious place to grab it from instead of
    /// hunting through `target/<triple>/release/...`.
    pub release: bool,
}

impl BuildContext {
    /// The `cargo build` output subdirectory this context's artifacts land
    /// in - `"release"` or `"debug"`, matching [`BuildContext::release`].
    pub fn profile_dir(&self) -> &'static str {
        if self.release { "release" } else { "debug" }
    }
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

    /// Starts streaming device/runtime logs and returns immediately (any
    /// actual log printing happens on a background thread) - `run.rs`'s
    /// hot-reload loop needs its main thread free to listen for the next
    /// `r`/`R`/`q` keypress right after this returns, not blocked reading
    /// logs until the process exits.
    fn stream_logs(&self, _ctx: &BuildContext, _start: std::time::Instant) -> Result<()> {
        Ok(())
    }

    /// Wipes whatever the last install left behind (uninstalling the APK,
    /// clearing app data, ...) before the next `build`/`run` - used for a
    /// full reload (`R` in `run.rs`'s hot-reload loop) to guarantee a truly
    /// clean slate; a quick reload (`r`) skips this and just reinstalls
    /// over the existing install, which is what makes it the fast path.
    fn full_reset(&self, _ctx: &BuildContext) -> Result<()> {
        Ok(())
    }

    /// A genuine in-process hot patch for `r` (quick reload) - rebuild just
    /// the changed code and swap it into a *still-running* app, no new
    /// process/window, no reinstall. Returns `Ok(true)` when it actually did
    /// this (`run.rs`'s hot-reload loop then skips the ordinary
    /// `build`+`run` entirely for that keypress); `Ok(false)` falls through
    /// to the ordinary path instead - the correct answer whenever there's no
    /// already-running instance to patch yet (the very first `r` right
    /// after startup) or the platform has no such mechanism at all. Only
    /// [`windows::WindowsTarget`](crate::targets::windows::WindowsTarget)
    /// implements this for now - see its own doc comment for how.
    fn quick_reload(&self, _ctx: &BuildContext) -> Result<bool> {
        Ok(false)
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
        &[
            Platform::Android,
            Platform::Ios,
            Platform::Web,
            Platform::Windows,
        ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_dir_is_release_or_debug() {
        let ctx = |release| BuildContext {
            manifest_dir: PathBuf::new(),
            target_triple: String::new(),
            crate_name: String::new(),
            lib_name: String::new(),
            target_directory: PathBuf::new(),
            release,
        };
        assert_eq!(ctx(true).profile_dir(), "release");
        assert_eq!(ctx(false).profile_dir(), "debug");
    }

    #[test]
    fn platform_as_str_round_trips_through_from_str() {
        for &p in Platform::all() {
            assert_eq!(Platform::from_str(p.as_str()).unwrap(), p);
        }
    }

    #[test]
    fn from_str_rejects_unknown_platforms() {
        assert!(Platform::from_str("nope").is_err());
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let all = Platform::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&Platform::Android));
        assert!(all.contains(&Platform::Ios));
        assert!(all.contains(&Platform::Web));
        assert!(all.contains(&Platform::Windows));
    }

    #[test]
    fn ios_is_not_implemented_others_are() {
        assert!(!Platform::Ios.handler().is_implemented());
        assert!(Platform::Android.handler().is_implemented());
        assert!(Platform::Web.handler().is_implemented());
        assert!(Platform::Windows.handler().is_implemented());
    }

    #[test]
    fn validate_triple_accepts_supported_and_rejects_unsupported() {
        let handler = Platform::Windows.handler();
        let supported = handler.supported_triples()[0];
        assert!(handler.validate_triple(supported).is_ok());
        assert!(handler.validate_triple("not-a-real-triple").is_err());
    }

    #[test]
    fn handler_id_matches_the_platform_it_was_created_from() {
        for &p in Platform::all() {
            assert_eq!(p.handler().id(), p);
        }
    }
}
