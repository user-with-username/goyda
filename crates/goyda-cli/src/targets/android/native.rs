use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

use super::AndroidAppLayout;
use crate::targets::BuildContext;
use crate::utils::run_command_quiet;

fn is_windows_host() -> bool {
    cfg!(windows)
}

fn check_cargo_ndk() -> Result<()> {
    let output = Command::new("cargo").args(&["ndk", "--version"]).output();

    match output {
        Ok(o) if o.status.success() => Ok(()),
        _ => bail!(
            "cargo-ndk not found. Install it with: cargo install cargo-ndk\n\
             Or build with --platform android and a target triple directly:\n\
             - Linux/Mac: cargo ndk --target <triple> ...\n\
             - Windows: cargo build --target <triple> ..."
        ),
    }
}

pub fn build_native_library(ctx: &BuildContext) -> Result<()> {
    if is_windows_host() {
        check_cargo_ndk()?;

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&ctx.manifest_dir)
            .args(&[
                "ndk",
                "--target",
                &ctx.target_triple,
                "--platform",
                "21",
                "build",
                "--lib",
                "--features",
                "goyda/android",
            ])
            .env("CARGO_NDK_ANDROID_PLATFORM", "21");

        run_command_quiet(&mut cmd, "building the Rust library via cargo-ndk failed")
    } else {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&ctx.manifest_dir).args(&[
            "build",
            "--target",
            &ctx.target_triple,
            "--lib",
            "--features",
            "goyda/android",
        ]);

        run_command_quiet(&mut cmd, "building the Rust library via cargo build failed")
    }
}

pub fn copy_native_library(ctx: &BuildContext, layout: &AndroidAppLayout) -> Result<()> {
    let so_filename = format!("lib{}.so", ctx.lib_name);

    let built_so_path: PathBuf = ctx
        .target_directory
        .join(&ctx.target_triple)
        .join("debug")
        .join(&so_filename);

    if !built_so_path.exists() {
        bail!("Compiled native library not found at: {:?}", built_so_path);
    }

    std::fs::copy(&built_so_path, layout.jni_libs_dir().join(&so_filename))
        .context("Failed to copy the compiled native library into the APK layout")?;
    Ok(())
}