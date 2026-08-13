use anyhow::{bail, Context, Result};
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::targets::{BuildContext, Platform};
use crate::term;

pub fn compile(platform: String, target: String, manifest_dir: PathBuf, release: bool) -> Result<()> {
    let start = Instant::now();
    let platform: Platform = platform.parse()?;
    let handler = platform.handler();

    if !handler.is_implemented() {
        let implemented: Vec<&str> = Platform::all()
            .iter()
            .filter(|p| p.handler().is_implemented())
            .map(Platform::as_str)
            .collect();
        bail!(
            "Platform '{}' is registered but not implemented yet. Supported platforms today: {}.",
            platform.as_str(),
            implemented.join(", ")
        );
    }

    handler.validate_triple(&target)?;

    let manifest_dir = fs::canonicalize(manifest_dir)?;
    let meta = MetadataCommand::new()
        .manifest_path(manifest_dir.join("Cargo.toml"))
        .exec()?;
    let root_package = meta
        .root_package()
        .context("No root package found in Cargo.toml")?;

    let crate_name = root_package.name.clone();
    let lib_name = root_package
        .targets
        .iter()
        .find(|t| t.kind.iter().any(|k| k == "lib"))
        .map(|t| t.name.clone())
        .unwrap_or_else(|| crate_name.replace('-', "_"));

    term::info(&format!("{crate_name} ({lib_name})"));

    let ctx = BuildContext {
        manifest_dir,
        target_triple: target,
        crate_name,
        lib_name,
        target_directory: meta.target_directory.into_std_path_buf(),
        start,
        release,
    };

    let artifact_path = handler.build(&ctx)?;

    let grab_path = if release {
        Some(copy_to_release_dir(&ctx, &artifact_path, platform.as_str())?)
    } else {
        None
    };

    match &grab_path {
        Some(p) => term::ready(&format!(" — {}", p.display()), start.elapsed()),
        None => term::ready(&format!(" — {}", artifact_path.display()), start.elapsed()),
    }
    Ok(())
}

/// Copies `artifact_path` (a single file for android/windows, a whole
/// directory of files for web) into `<manifest_dir>/release/<platform>/` -
/// a stable, obvious place to grab a `--release` build from instead of
/// hunting through `target/<triple>/release/...`, which differs per
/// platform and (for android/windows) isn't even the final packaged
/// artifact's own location to begin with (that's the platform-specific
/// app-layout build dir under `target/goyda_*`).
fn copy_to_release_dir(ctx: &BuildContext, artifact_path: &std::path::Path, platform: &str) -> Result<PathBuf> {
    let dest_dir = ctx.manifest_dir.join("release").join(platform);
    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir).context("Failed to clear the previous release/ output")?;
    }
    fs::create_dir_all(&dest_dir).context("Failed to create the release/ output directory")?;

    if artifact_path.is_dir() {
        crate::utils::copy_dir_recursive(artifact_path, &dest_dir)
            .context("Failed to copy the build output into release/")?;
        Ok(dest_dir)
    } else {
        let file_name = artifact_path
            .file_name()
            .context("Build artifact has no file name")?;
        let dest_file = dest_dir.join(file_name);
        fs::copy(artifact_path, &dest_file).context("Failed to copy the build artifact into release/")?;
        Ok(dest_file)
    }
}