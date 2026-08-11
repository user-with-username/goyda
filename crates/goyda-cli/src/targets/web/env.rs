use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::GOYDA_DIR_NAME;
use crate::utils::{find_tool, run_command};

pub struct WebBuildEnvironment {
    pub wasm_bindgen_bin: PathBuf,
}

impl WebBuildEnvironment {
    /// Ensures the `wasm32-unknown-unknown` rustup target and a
    /// version-matched `wasm-bindgen` CLI are available, installing whatever
    /// is missing. The CLI version must match the `wasm-bindgen` crate
    /// version resolved for the project exactly, or it refuses to process
    /// the wasm module - so a globally installed `wasm-bindgen` is only
    /// reused if its version already matches; otherwise a private,
    /// version-pinned copy is installed under `~/.goyda/tools`.
    pub fn prepare(required_version: &str) -> Result<Self> {
        ensure_wasm_target_installed()?;

        if let Ok(path) = find_tool("wasm-bindgen") {
            if installed_version(&path).as_deref() == Some(required_version) {
                return Ok(Self { wasm_bindgen_bin: path });
            }
        }

        let home_dir = dirs::home_dir().context("Could not locate the user's home directory")?;
        let tool_root = home_dir
            .join(GOYDA_DIR_NAME)
            .join("tools")
            .join(format!("wasm-bindgen-{required_version}"));

        let bin_name = if cfg!(windows) { "wasm-bindgen.exe" } else { "wasm-bindgen" };
        let wasm_bindgen_bin = tool_root.join("bin").join(bin_name);

        if !wasm_bindgen_bin.exists() {
            install_wasm_bindgen_cli(required_version, &tool_root)?;
        }

        if !wasm_bindgen_bin.exists() {
            anyhow::bail!(
                "wasm-bindgen-cli {required_version} was installed but the binary was not found at {:?}",
                wasm_bindgen_bin
            );
        }

        Ok(Self { wasm_bindgen_bin })
    }
}

fn installed_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    // Output looks like "wasm-bindgen 0.2.127".
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .rsplit(' ')
        .next()
        .map(|s| s.to_string())
}

fn ensure_wasm_target_installed() -> Result<()> {
    let listed = Command::new("rustup").args(&["target", "list", "--installed"]).output();

    let already_installed = matches!(
        &listed,
        Ok(o) if o.status.success()
            && String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == "wasm32-unknown-unknown")
    );

    if already_installed {
        return Ok(());
    }

    let mut cmd = Command::new("rustup");
    cmd.args(&["target", "add", "wasm32-unknown-unknown"]);
    run_command(&mut cmd, "Failed to install the wasm32-unknown-unknown target via rustup")
}

fn install_wasm_bindgen_cli(version: &str, tool_root: &Path) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(&["install", "wasm-bindgen-cli", "--version", version, "--locked", "--root"]);
    cmd.arg(tool_root);
    run_command(
        &mut cmd,
        "Failed to install wasm-bindgen-cli. Try installing it manually: \
         cargo install wasm-bindgen-cli --version <version>",
    )
}
