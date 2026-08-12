use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::time::Instant;

use crate::targets::{BuildContext, Platform};

pub struct RunOptions {
    pub platform: Platform,
    pub target_triple: Option<String>,
    pub manifest_dir: PathBuf,
}

pub fn execute_run(opts: RunOptions) -> Result<()> {
    let start = Instant::now();
    let handler = opts.platform.handler();

    if !handler.is_implemented() {
        bail!(
            "Target platform '{}' is not implemented yet",
            opts.platform.as_str()
        );
    }

    let triple = opts.target_triple.unwrap_or_else(|| {
        handler.supported_triples()[0].to_string()
    });

    handler.validate_triple(&triple)?;

    let manifest_path = opts.manifest_dir.join("Cargo.toml");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest file at {:?}", manifest_path))?;

    let cargo_toml: cargo_toml::Manifest = cargo_toml::Manifest::from_slice(manifest_content.as_bytes())
        .with_context(|| format!("Failed to parse Cargo.toml at {:?}", manifest_path))?;

    let package = cargo_toml
        .package
        .ok_or_else(|| anyhow::anyhow!("Missing [package] section in Cargo.toml"))?;

    let crate_name = package.name;
    let lib_name = crate_name.replace('-', "_");

    let ctx = BuildContext {
        manifest_dir: opts.manifest_dir.clone(),
        target_triple: triple,
        crate_name,
        lib_name,
        target_directory: opts.manifest_dir.join("target"),
        start,
    };

    let artifact_path = handler
        .build(&ctx)
        .context("Failed to build project before running")?;

    if !artifact_path.exists() {
        bail!("Build artifact was not generated at expected path: {:?}", artifact_path);
    }

    handler
        .run(&ctx, &artifact_path)
        .context("Failed to execute application on target device")?;

    handler
        .stream_logs(&ctx, start)
        .context("Failed to stream logs from device")?;

    Ok(())
}