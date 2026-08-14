use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::utils::run_command;

pub fn run_wasm_bindgen(
    bin: &Path,
    wasm_path: &Path,
    out_dir: &Path,
    out_name: &str,
) -> Result<()> {
    let mut cmd = Command::new(bin);
    cmd.args([
        "--target",
        "web",
        "--out-dir",
        out_dir.to_str().context("Invalid output directory path")?,
        "--out-name",
        out_name,
        "--no-typescript",
        wasm_path.to_str().context("Invalid wasm artifact path")?,
    ]);

    run_command(
        &mut cmd,
        "wasm-bindgen failed to generate the JS/wasm bindings",
    )
}
