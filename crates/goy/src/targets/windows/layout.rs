use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Everything a windows build needs on disk. Unlike the old build (one
/// statically-linked runner exe with the consumer crate baked in), this now
/// has to keep two independently rebuildable pieces straight - the
/// persistent host and the consumer's reloadable `cdylib` - and, critically,
/// build *both* through one single `cargo build --workspace` invocation (see
/// [`workspace_dir`]/`build.rs`'s doc comments) rather than two separate
/// ones: `rustc` bakes a per-compilation-unit disambiguator hash into every
/// symbol name, and that hash is *not* guaranteed to match across two
/// separate `cargo build` processes even with an identical `Cargo.lock` and
/// `--target-dir` - which silently produces a host `.exe` whose import table
/// references a `goyda.dll` export name that the *actual* `goyda.dll` on
/// disk doesn't have (a real, reproduced failure during this rewrite -
/// `STATUS_DLL_NOT_FOUND` at process start, confirmed via `dumpbin
/// /exports` vs `/imports` showing two different disambiguator hashes for
/// the same `goyda::windows::run` symbol). One workspace, one `cargo build`
/// call, one compiled `goyda` unit shared by everything in it - that's the
/// only way to actually guarantee this.
pub struct WindowsAppLayout {
    workspace_dir: PathBuf,
    shared_target_dir: PathBuf,
    bin_dir: PathBuf,
    final_exe: PathBuf,
    assets_dir: PathBuf,
}

impl WindowsAppLayout {
    pub fn new(manifest_dir: &Path, crate_name: &str) -> Self {
        let build_dir = manifest_dir.join("target").join("goyda_windows");
        let bin_dir = build_dir.join("bin");
        Self {
            workspace_dir: build_dir.join("workspace"),
            // Deliberately *not* nested under `build_dir` (which already
            // sits under the project's own, often long, path): rustc's
            // object file paths for deeply nested build-script/proc-macro
            // outputs can exceed `MAX_PATH`, tripping rustc into emitting
            // `\\?\`-prefixed extended-length paths that the mingw-w64 `ld`
            // this toolchain uses doesn't parse correctly (it mistakes the
            // prefix for a UNC share and drops the drive letter, producing
            // "cannot find \\symbols.o" link failures). A short, flat path
            // under the system temp dir keyed by crate name sidesteps this
            // - shared across every build of the same project, not
            // recreated per invocation.
            shared_target_dir: std::env::temp_dir().join("goyda_win").join(crate_name),
            final_exe: bin_dir.join(format!("{crate_name}.exe")),
            bin_dir,
            assets_dir: build_dir.join("assets"),
        }
    }

    pub fn init_directories(&self) -> Result<()> {
        fs::create_dir_all(self.workspace_dir.join("host").join("src"))?;
        fs::create_dir_all(self.workspace_dir.join("shim").join("src"))?;
        fs::create_dir_all(&self.bin_dir)?;
        Ok(())
    }

    /// The synthetic cargo workspace root - see this struct's own doc
    /// comment for why the host and the consumer's dylib-building "shim"
    /// (see `build.rs`'s [`write_consumer_shim`](super::build)) have to be
    /// members of *one* workspace instead of two independently-built
    /// crates.
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }
    pub fn host_dir(&self) -> PathBuf {
        self.workspace_dir.join("host")
    }
    pub fn shim_dir(&self) -> PathBuf {
        self.workspace_dir.join("shim")
    }
    pub fn shared_target_dir(&self) -> &Path {
        &self.shared_target_dir
    }
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }
    pub fn final_exe(&self) -> &Path {
        &self.final_exe
    }
    pub fn assets_dir(&self) -> &Path {
        &self.assets_dir
    }
}
