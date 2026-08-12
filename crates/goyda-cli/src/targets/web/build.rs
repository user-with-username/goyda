use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::PathBuf;
use std::process::Command;

use crate::targets::BuildContext;
use crate::term;
use crate::utils::{copy_dir_recursive, run_command_quiet};

use super::env::WebBuildEnvironment;
use super::layout::WebAppLayout;
use super::wasm_bindgen::run_wasm_bindgen;

const INDEX_HTML_TEMPLATE: &str = include_str!("../../../templates/index.html");

pub fn build_web_app(ctx: &BuildContext) -> Result<PathBuf> {
    let layout = WebAppLayout::new(&ctx.manifest_dir);
    layout.init_directories()?;

    let wasm_bindgen_version = resolve_wasm_bindgen_version(ctx)?;

    let s = term::spinner_step("dev toolchain");
    let env = match WebBuildEnvironment::prepare(&wasm_bindgen_version) {
        Ok(env) => {
            s.ok();
            env
        }
        Err(e) => {
            s.fail(&e.to_string());
            return Err(e);
        }
    };

    let s = term::spinner_step("native library");
    let compiled = compile_to_wasm(ctx);
    match &compiled {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    compiled?;

    let wasm_path = ctx
        .target_directory
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{}.wasm", ctx.lib_name));

    if !wasm_path.exists() {
        anyhow::bail!("Compiled wasm module not found at: {:?}", wasm_path);
    }

    let s = term::spinner_step("generate bindings");
    let bound = run_wasm_bindgen(&env.wasm_bindgen_bin, &wasm_path, layout.dist_dir(), &ctx.lib_name);
    match &bound {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    bound?;

    let s = term::step("layout & assets");
    write_index_html(&layout, &ctx.crate_name, &ctx.lib_name)?;
    s.ok();

    let s = term::step("project assets");
    copy_dir_recursive(&ctx.manifest_dir.join("assets"), &layout.dist_dir().join("assets"))
        .context("Failed to copy the assets/ directory into the web build")?;
    s.ok();

    Ok(layout.dist_dir().to_path_buf())
}

fn resolve_wasm_bindgen_version(ctx: &BuildContext) -> Result<String> {
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path(ctx.manifest_dir.join("Cargo.toml"));
    cmd.other_options(vec![
        "--no-default-features".to_string(),
        "--features".to_string(),
        "goyda/web".to_string(),
        "--filter-platform".to_string(),
        ctx.target_triple.clone(),
    ]);

    let meta = cmd
        .exec()
        .context("Failed to resolve the wasm-bindgen version via `cargo metadata`")?;

    meta.packages
        .iter()
        .find(|p| p.name == "wasm-bindgen")
        .map(|p| p.version.to_string())
        .context(
            "Could not find `wasm-bindgen` in the resolved dependency graph. Make sure the \
             `goyda` dependency in Cargo.toml keeps default features on (or explicitly \
             re-enables `web`), and that crates.io is reachable.",
        )
}

fn compile_to_wasm(ctx: &BuildContext) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ctx.manifest_dir).args(&[
        "build",
        "--target",
        &ctx.target_triple,
        "--lib",
        "--no-default-features",
        "--features",
        "goyda/web",
    ]);

    run_command_quiet(&mut cmd, "building the Rust library for wasm32 via cargo build failed")
}

fn write_index_html(layout: &WebAppLayout, app_name: &str, out_name: &str) -> Result<()> {
    let rendered = INDEX_HTML_TEMPLATE
        .replace("{app_name}", app_name)
        .replace("{out_name}", out_name);

    std::fs::write(layout.dist_dir().join("index.html"), rendered).context("Failed to write index.html")
}
