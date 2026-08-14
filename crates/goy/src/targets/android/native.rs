use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::AndroidAppLayout;
use crate::targets::BuildContext;
use crate::utils::run_command_quiet;

/// `-C prefer-dynamic` makes the consumer crate link against `goyda`'s
/// `dylib` output (`libgoyda.so`, from `[lib] crate-type = ["rlib",
/// "dylib"]` in `crates/goyda/Cargo.toml`) instead of statically compiling
/// its own private copy - exactly the same trick (and for exactly the same
/// reason - see that Cargo.toml comment, and `windows/build.rs`'s
/// `PREFER_DYNAMIC` doc comment) used for windows hot reload. Android needs
/// no separate "host" binary the way windows did: `goyda`'s own JNI entry
/// point (`JNI_OnLoad`, in `android/bootstrap.rs`) already lives inside
/// `goyda` itself, so building the consumer crate through this *one* cargo-
/// ndk invocation compiles `goyda` and the consumer together in the same
/// unit graph - no risk of the two disagreeing on `goyda`'s compiled
/// symbol names the way two *separate* invocations could (a real bug hit
/// and fixed during the windows implementation).
const PREFER_DYNAMIC: &str = "-C prefer-dynamic";

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

        let mut args = vec![
            "ndk",
            "--target",
            &ctx.target_triple,
            "--platform",
            "21",
            "build",
            "--lib",
            "--features",
            "goyda/android",
        ];
        if ctx.release {
            args.push("--release");
        }

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&ctx.manifest_dir)
            .args(&args)
            .env("CARGO_NDK_ANDROID_PLATFORM", "21")
            .env("RUSTFLAGS", PREFER_DYNAMIC);

        run_command_quiet(&mut cmd, "building the Rust library via cargo-ndk failed")
    } else {
        let mut args = vec![
            "build",
            "--target",
            &ctx.target_triple,
            "--lib",
            "--features",
            "goyda/android",
        ];
        if ctx.release {
            args.push("--release");
        }

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&ctx.manifest_dir)
            .args(&args)
            .env("RUSTFLAGS", PREFER_DYNAMIC);

        run_command_quiet(&mut cmd, "building the Rust library via cargo build failed")
    }
}

/// Finds `libstd-*.so` in the active toolchain's target sysroot - `-C
/// prefer-dynamic` links against it dynamically too (not just
/// `libgoyda.so`), so it has to be packaged into the APK alongside the
/// other native libraries same as any other runtime dependency.
fn find_std_dylib(target_triple: &str) -> Result<Option<PathBuf>> {
    let output = Command::new("rustc")
        .args(&["--print", "target-libdir", "--target", target_triple])
        .output()
        .context("Failed to run `rustc --print target-libdir`")?;
    if !output.status.success() {
        return Ok(None);
    }
    let libdir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if !libdir.is_dir() {
        return Ok(None);
    }
    for entry in fs::read_dir(&libdir).with_context(|| format!("Failed to read {:?}", libdir))? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("libstd-") && name.ends_with(".so") {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

pub fn copy_native_library(ctx: &BuildContext, layout: &AndroidAppLayout) -> Result<()> {
    let profile_dir = ctx
        .target_directory
        .join(&ctx.target_triple)
        .join(ctx.profile_dir());

    let so_filename = format!("lib{}.so", ctx.lib_name);
    let built_so_path = profile_dir.join(&so_filename);
    if !built_so_path.exists() {
        bail!("Compiled native library not found at: {:?}", built_so_path);
    }
    std::fs::copy(&built_so_path, layout.jni_libs_dir().join(&so_filename))
        .context("Failed to copy the compiled native library into the APK layout")?;

    // `-C prefer-dynamic` (see `build_native_library`) means the consumer
    // library above only *links against* `goyda` and `std` - it doesn't
    // contain them. Both have to ride along in the APK too, or the app
    // fails to even start (`dlopen` can't resolve `DT_NEEDED libgoyda.so`).
    let goyda_so_path = profile_dir.join("libgoyda.so");
    if !goyda_so_path.exists() {
        bail!("Compiled libgoyda.so not found at: {:?}", goyda_so_path);
    }
    std::fs::copy(&goyda_so_path, layout.jni_libs_dir().join("libgoyda.so"))
        .context("Failed to copy libgoyda.so into the APK layout")?;

    if let Some(std_so_path) = find_std_dylib(&ctx.target_triple)? {
        let dest = layout.jni_libs_dir().join(std_so_path.file_name().unwrap());
        std::fs::copy(&std_so_path, &dest)
            .context("Failed to copy the dynamic std runtime into the APK layout")?;
    }

    Ok(())
}
