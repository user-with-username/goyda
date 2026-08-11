use anyhow::{bail, Context, Result};
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::targets::{BuildContext, Platform};
use crate::term;

pub fn compile(platform: String, target: String, manifest_dir: PathBuf) -> Result<()> {
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
    };

    let artifact_path = handler.build(&ctx)?;

    term::ready(&format!(" — {}", artifact_path.display()), start.elapsed());
    Ok(())
}