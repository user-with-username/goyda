use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::targets::BuildContext;
use crate::term;
use crate::utils::{copy_dir_recursive, run_command_quiet};

use super::layout::WindowsAppLayout;

pub fn build_windows_app(ctx: &BuildContext) -> Result<PathBuf> {
    let layout = WindowsAppLayout::new(&ctx.manifest_dir, &ctx.crate_name);
    layout.init_directories()?;

    ensure_target_installed(&ctx.target_triple)?;

    let s = term::step("layout & assets");
    let goyda_dir = resolve_goyda_manifest_dir(ctx)?;
    write_runner_crate(&layout, ctx, &goyda_dir)?;
    s.ok();

    let s = term::spinner_step("native library");
    let compiled = compile_runner(&layout, ctx);
    match &compiled {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    compiled?;

    let s = term::step("project assets");
    copy_dir_recursive(&ctx.manifest_dir.join("assets"), layout.assets_dir())
        .context("Failed to copy the assets/ directory into the windows build")?;
    s.ok();

    let s = term::spinner_step("package exe");
    let packaged = package_exe(&layout, ctx);
    match &packaged {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    packaged?;

    Ok(layout.final_exe().to_path_buf())
}

fn ensure_target_installed(triple: &str) -> Result<()> {
    let listed = Command::new("rustup").args(&["target", "list", "--installed"]).output();

    let already_installed = matches!(
        &listed,
        Ok(o) if o.status.success()
            && String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == triple)
    );
    if already_installed {
        return Ok(());
    }

    let mut cmd = Command::new("rustup");
    cmd.args(&["target", "add", triple]);
    run_command_quiet(&mut cmd, "Failed to install the windows target via rustup")
}

/// Resolves where the `goyda` crate the consumer project actually depends
/// on lives on disk, so the generated runner crate (see
/// [`write_runner_crate`]) can depend on that exact same `goyda` (by path)
/// instead of guessing a version - avoiding two different copies of `goyda`
/// ending up in the same build.
fn resolve_goyda_manifest_dir(ctx: &BuildContext) -> Result<PathBuf> {
    let meta = MetadataCommand::new()
        .manifest_path(ctx.manifest_dir.join("Cargo.toml"))
        .exec()
        .context("Failed to resolve the project's dependency graph via `cargo metadata`")?;

    let goyda = meta
        .packages
        .iter()
        .find(|p| p.name == "goyda")
        .context("Could not find `goyda` in the resolved dependency graph")?;

    Ok(goyda
        .manifest_path
        .parent()
        .context("goyda's manifest path has no parent directory")?
        .as_std_path()
        .to_path_buf())
}

/// The consumer crate builds as a library only (`crate-type = ["cdylib",
/// "rlib"]`, shared with the web/android targets) - it has no `fn main()`.
/// Unlike wasm (`#[wasm_bindgen(start)]`) or Android (`JNI_OnLoad`, called
/// automatically by the OS loader), a runnable `.exe` needs an actual PE
/// entry point that only a `[[bin]]` crate produces, so this generates a
/// tiny separate crate that depends on the consumer crate as a library and
/// hands off to [`goyda::windows::run`](crate) as soon as `main` starts.
/// `codegen-units = 1` plus directly referencing the consumer crate (rather
/// than just listing it as a dependency) makes sure its `#[page(...)]`
/// registrations actually get linked in - an unreferenced dependency's
/// object code can otherwise be dropped by the linker even though Cargo
/// built it.
fn write_runner_crate(layout: &WindowsAppLayout, ctx: &BuildContext, goyda_dir: &Path) -> Result<()> {
    let manifest_dir = normalize(&ctx.manifest_dir)?;
    let goyda_dir = normalize(goyda_dir)?;

    let cargo_toml = format!(
        r#"[package]
name = "goyda_windows_runner"
version = "0.0.0"
edition = "2021"
publish = false

[[bin]]
name = "{crate_name}"
path = "src/main.rs"

[dependencies]
{crate_name} = {{ path = "{manifest_dir}" }}
goyda = {{ path = "{goyda_dir}", default-features = false, features = ["windows"] }}

[profile.dev]
codegen-units = 1

[profile.release]
codegen-units = 1
"#,
        crate_name = ctx.crate_name,
        manifest_dir = manifest_dir,
        goyda_dir = goyda_dir,
    );
    fs::write(layout.runner_dir().join("Cargo.toml"), cargo_toml).context("Failed to write the windows runner crate's Cargo.toml")?;

    let lib_ident = ctx.lib_name.replace('-', "_");
    let main_rs = format!(
        r#"extern crate {lib_ident} as _consumer_crate;

fn main() {{
    goyda::windows::run();
}}
"#
    );
    fs::write(layout.runner_dir().join("src").join("main.rs"), main_rs)
        .context("Failed to write the windows runner crate's main.rs")?;

    Ok(())
}

/// TOML string escaping doesn't need to special-case backslashes if there
/// simply aren't any - Windows paths work fine with forward slashes, and it
/// sidesteps having to escape the path for a TOML string literal.
fn normalize(path: &Path) -> Result<String> {
    Ok(crate::utils::normalize_path(path)?.replace('\\', "/"))
}

fn compile_runner(layout: &WindowsAppLayout, ctx: &BuildContext) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(layout.runner_dir()).args(&["build", "--target", &ctx.target_triple]);
    run_command_quiet(&mut cmd, "building the windows executable via cargo build failed")
}

fn package_exe(layout: &WindowsAppLayout, ctx: &BuildContext) -> Result<()> {
    let built_exe = layout
        .runner_dir()
        .join("target")
        .join(&ctx.target_triple)
        .join("debug")
        .join(format!("{}.exe", ctx.crate_name));

    if !built_exe.exists() {
        anyhow::bail!("Compiled executable not found at: {:?}", built_exe);
    }
    fs::copy(&built_exe, layout.final_exe()).context("Failed to copy the built .exe into the windows build output")?;
    Ok(())
}
