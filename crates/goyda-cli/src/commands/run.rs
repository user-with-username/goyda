use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::targets::{BuildContext, Platform};
use crate::term;

pub struct RunOptions {
    pub platform: Platform,
    pub target_triple: Option<String>,
    pub manifest_dir: PathBuf,
    pub release: bool,
}

/// Restores the terminal's normal line-buffered/echoing mode on drop, so a
/// build error or `q`/Ctrl+C exit can't leave the user's shell stuck in
/// raw mode (no visible input, no Enter-to-submit) after `run` exits.
struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self> {
        enable_raw_mode().context("Failed to enable terminal raw mode for hot-reload keys")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub fn execute_run(opts: RunOptions) -> Result<()> {
    let handler = opts.platform.handler();

    if !handler.is_implemented() {
        bail!(
            "Target platform '{}' is not implemented yet",
            opts.platform.as_str()
        );
    }

    let triple = opts
        .target_triple
        .clone()
        .unwrap_or_else(|| handler.supported_triples()[0].to_string());
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

    // `full_reset` is the only difference between `r` (quick reload - just
    // reinstall over the existing app) and `R` (full reload - uninstall
    // first, guaranteeing nothing from the previous install lingers). Note
    // on scope: this rebuilds and reinstalls fast (cargo's own incremental
    // compilation already makes an unchanged-dependencies rebuild quick),
    // but it's still a real process restart - in-memory UI state (typed
    // text, counters, ...) is not preserved across either kind of reload,
    // only the app being back on screen quickly is what's "smart" here.
    let build_and_run = |full_reset: bool| -> Result<()> {
        let start = Instant::now();
        let ctx = BuildContext {
            manifest_dir: opts.manifest_dir.clone(),
            target_triple: triple.clone(),
            crate_name: crate_name.clone(),
            lib_name: lib_name.clone(),
            target_directory: opts.manifest_dir.join("target"),
            start,
            release: opts.release,
        };

        if full_reset {
            handler
                .full_reset(&ctx)
                .context("Failed to reset the previous install")?;
        } else if handler.quick_reload(&ctx).context("Quick reload failed")? {
            return Ok(());
        }

        let artifact_path = handler
            .build(&ctx)
            .context("Failed to build project before running")?;

        if !artifact_path.exists() {
            bail!(
                "Build artifact was not generated at expected path: {:?}",
                artifact_path
            );
        }

        handler
            .run(&ctx, &artifact_path)
            .context("Failed to execute application on target device")?;

        handler
            .stream_logs(&ctx, start)
            .context("Failed to stream logs from device")?;

        Ok(())
    };

    build_and_run(false)?;

    term::info("r reload (quick) · R reload (full) · q / Ctrl+C quit");

    let _raw_mode = RawModeGuard::enable()?;
    loop {
        let Event::Key(key) = event::read().context("Failed to read a terminal key event")? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('r') => {
                term::header("reload", &[]);
                if let Err(e) = build_and_run(false) {
                    term::info(&format!("reload failed: {e:#}"));
                }
            }
            KeyCode::Char('R') => {
                term::header("full reload", &[]);
                if let Err(e) = build_and_run(true) {
                    term::info(&format!("full reload failed: {e:#}"));
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Char('q') => return Ok(()),
            _ => {}
        }
    }
}
