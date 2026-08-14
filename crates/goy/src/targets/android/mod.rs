use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use crate::targets::{BuildContext, Platform, PlatformTarget};
use crate::term;
use crate::utils::find_tool;

mod builder;
mod env;
mod layout;
mod listeners;
mod native;
mod templates;

pub use env::BuildEnvironment;
pub use layout::AndroidAppLayout;

pub struct AndroidTarget;

impl PlatformTarget for AndroidTarget {
    fn id(&self) -> Platform {
        Platform::Android
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &[
            "aarch64-linux-android",
            "armv7-linux-androideabi",
            "i686-linux-android",
            "x86_64-linux-android",
        ]
    }

    fn build(&self, ctx: &BuildContext) -> Result<PathBuf> {
        term::header("UI", &[&ctx.target_triple]);

        let env = BuildEnvironment::prepare()?;

        let layout = AndroidAppLayout::new(&ctx.manifest_dir, &ctx.target_triple, &ctx.crate_name)?;
        layout.init_directories()?;

        let s = term::spinner_step("native library");
        let result = native::build_native_library(ctx)
            .and_then(|_| native::copy_native_library(ctx, &layout));
        match result {
            Ok(()) => s.ok(),
            Err(e) => {
                s.fail(&e.to_string());
                return Err(e);
            }
        }

        let s = term::step("layout & assets");
        templates::generate_assets_from_templates(
            &layout,
            &ctx.crate_name,
            &ctx.lib_name,
            ctx.release,
        )?;
        listeners::write_runtime_listener_classes(&layout)?;
        s.ok();

        let s = term::step("project assets");
        crate::utils::copy_dir_recursive(&ctx.manifest_dir.join("assets"), layout.assets_dir())
            .context("Failed to copy the assets/ directory into the APK")?;
        s.ok();

        let s = term::spinner_step("package apk");
        match builder::build_apk_package(&layout, &env) {
            Ok(()) => s.ok(),
            Err(e) => {
                s.fail(&e.to_string());
                return Err(e);
            }
        }

        Ok(layout.final_apk().to_path_buf())
    }

    fn run(&self, ctx: &BuildContext, artifact_path: &Path) -> Result<()> {
        let adb_path = find_tool("adb")
            .context("adb tool not found. Make sure Android SDK platform-tools are in PATH")?;

        let s = term::spinner_step("install & launch");
        let result = deploy(&adb_path, ctx, artifact_path);

        match &result {
            Ok(()) => s.ok(),
            Err(e) => s.fail(&e.to_string()),
        }

        result
    }

    fn stream_logs(&self, ctx: &BuildContext, _start: Instant) -> Result<()> {
        let crate_name = ctx.crate_name.clone();
        thread::spawn(move || {
            let _ = stream_android_logs(&crate_name);
        });
        Ok(())
    }

    fn full_reset(&self, ctx: &BuildContext) -> Result<()> {
        let adb_path = find_tool("adb")
            .context("adb tool not found. Make sure Android SDK platform-tools are in PATH")?;
        let package_name = format!("com.goyda.{}", ctx.crate_name);
        // `adb uninstall` failing (nothing installed yet, e.g. the very
        // first run) isn't an error worth stopping a reload over. This also
        // wipes the app's private storage - and with it every
        // `app_gen*.so` a previous session's quick reloads pushed there -
        // so generation numbering starts clean at 0 again too.
        let _ = Command::new(&adb_path)
            .args(["uninstall", &package_name])
            .output();
        Ok(())
    }

    /// The real hot-reload path: if the app is already running, this
    /// rebuilds *only* the consumer crate's `.so` (fast - `goyda` itself is
    /// unchanged, so cargo's own incremental build reuses its already-
    /// compiled, already-loaded-by-the-running-app `libgoyda.so` from
    /// cache), pushes it into the app's own private storage under a fresh
    /// generation filename, and broadcasts to
    /// `HotReloadSwapReceiver` (see the `HotReloadSwapReceiver.java`/
    /// `AndroidManifest.xml` templates) to `System.load()` it into that
    /// *already-running* process - no reinstall, no restart. Falls through
    /// to the ordinary `build`+`run` (returns `Ok(false)`) whenever the app
    /// isn't running yet to patch, e.g. the very first `r` right after
    /// `goy run android` starts.
    fn quick_reload(&self, ctx: &BuildContext) -> Result<bool> {
        let adb_path = find_tool("adb")
            .context("adb tool not found. Make sure Android SDK platform-tools are in PATH")?;
        let package_name = format!("com.goyda.{}", ctx.crate_name);

        if find_running_pid(&adb_path, &package_name)?.is_none() {
            return Ok(false);
        }

        native::build_native_library(ctx)?;

        let built_so_path = ctx
            .target_directory
            .join(&ctx.target_triple)
            .join(ctx.profile_dir())
            .join(format!("lib{}.so", ctx.lib_name));
        if !built_so_path.exists() {
            bail!(
                "Compiled consumer library not found at: {:?}",
                built_so_path
            );
        }

        let device_path = push_next_generation(&adb_path, &package_name, &built_so_path)?;
        send_hot_swap(&adb_path, &package_name, &device_path)?;

        Ok(true)
    }
}

/// A single, non-blocking check (unlike [`wait_for_pid_via_ps`], which
/// retries for up to a timeout - appropriate right after a fresh install,
/// but wrong here: a *missing* process on `r` isn't a transient race to
/// wait out, it's the normal "nothing running yet, fall through to an
/// ordinary build+run" case) for whether `package` already has a running
/// process - what [`AndroidTarget::quick_reload`] gates on.
fn find_running_pid(adb_path: &Path, package: &str) -> Result<Option<String>> {
    let output = Command::new(adb_path)
        .args(["shell", "ps", "-A", "-o", "PID,NAME"])
        .output()
        .context("Failed to run ps on device")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == package {
            return Ok(Some(parts[0].to_string()));
        }
    }
    Ok(None)
}

/// Pushes `local_so` into `package`'s private storage (`adb push` alone
/// can't write there directly - `run-as` is what actually grants that
/// access, only available because the manifest sets `android:debuggable`
/// for non-`--release` builds - see `templates.rs`) under the next unused
/// `app_gen{N}.so` filename, mirroring windows' own generation-numbered
/// dylibs (see `windows/build.rs::next_generation_path`) for the same
/// reason: `System.load()`ing over an already-loaded generation's file
/// isn't safe, so each reload needs its own never-before-used name.
/// Returns the full on-device path, ready to hand to [`send_hot_swap`].
fn push_next_generation(adb_path: &Path, package: &str, local_so: &Path) -> Result<String> {
    // `files/` doesn't exist yet on a freshly installed app that's never
    // called `Context.getFilesDir()` (goyda's android backend never does) -
    // a no-op if it's already there.
    let _ = Command::new(adb_path)
        .args(["shell", "run-as", package, "mkdir", "-p", "files"])
        .output();

    let list_output = Command::new(adb_path)
        .args(["shell", "run-as", package, "ls", "files/"])
        .output()
        .context("Failed to list the app's private storage via run-as")?;
    let listing = String::from_utf8_lossy(&list_output.stdout);

    let next_generation = listing
        .lines()
        .filter_map(|name| {
            name.trim()
                .strip_prefix("app_gen")?
                .strip_suffix(".so")?
                .parse::<u32>()
                .ok()
        })
        .max()
        .map_or(0, |m| m + 1);

    let device_filename = format!("app_gen{next_generation}.so");
    let tmp_path = format!("/data/local/tmp/{device_filename}");
    let final_path = format!("/data/data/{package}/files/{device_filename}");

    run_adb_quiet(
        adb_path,
        &[
            "push",
            local_so.to_str().context("Invalid .so path")?,
            &tmp_path,
        ],
        "adb push of the rebuilt native library failed",
    )?;
    run_adb_quiet(
        adb_path,
        &["shell", "run-as", package, "cp", &tmp_path, &final_path],
        "copying the rebuilt native library into the app's private storage failed",
    )?;
    let _ = Command::new(adb_path)
        .args(["shell", "rm", &tmp_path])
        .output();

    Ok(final_path)
}

/// Explicit-component-targeted (`-n`, not `-a`/`-p`) broadcast to
/// `HotReloadSwapReceiver` carrying `device_path` as a string extra - see
/// that receiver's own doc comment for why this has to be explicit
/// (implicit broadcasts to manifest-declared receivers are silently
/// dropped since Android 8), a real bug hit and fixed earlier in this same
/// project's hot-reload work.
fn send_hot_swap(adb_path: &Path, package: &str, device_path: &str) -> Result<()> {
    let receiver_component = format!("{package}/.HotReloadSwapReceiver");
    let action = format!("{package}.HOT_SWAP");
    run_adb_quiet(
        adb_path,
        &[
            "shell",
            "am",
            "broadcast",
            "-n",
            &receiver_component,
            "-a",
            &action,
            "--es",
            "path",
            device_path,
        ],
        "broadcasting the hot-swap trigger failed",
    )
}

fn run_adb_quiet(adb_path: &Path, args: &[&str], context_msg: &str) -> Result<()> {
    let output = Command::new(adb_path)
        .args(args)
        .output()
        .with_context(|| format!("{context_msg}: failed to spawn adb"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        bail!("{context_msg}: {}", detail.trim());
    }

    Ok(())
}

fn deploy(adb_path: &Path, ctx: &BuildContext, artifact_path: &Path) -> Result<()> {
    let output = Command::new(adb_path)
        .arg("devices")
        .output()
        .context("Failed to query adb devices")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices: Vec<&str> = stdout
        .lines()
        .skip(1)
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.ends_with("device"))
        .collect();

    if devices.is_empty() {
        bail!("no running emulator or connected device found");
    }

    let package_name = format!("com.goyda.{}", ctx.crate_name);

    let apk_path = artifact_path.to_str().context("Invalid APK path")?;
    run_adb_quiet(adb_path, &["install", "-r", apk_path], "adb install failed")?;

    let activity_component = format!("{}/.MainActivity", package_name);

    run_adb_quiet(
        adb_path,
        &["shell", "am", "start", "-n", &activity_component],
        "adb activity launch failed",
    )?;

    Ok(())
}

fn parse_log_level(line: &str) -> Option<(char, &str)> {
    // "08-07 16:44:28.125  6673  6673 D VRI[MainActivity]: ..."
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 5 {
        let level_candidate = parts[4];
        if level_candidate.len() == 1 {
            let ch = level_candidate.chars().next().unwrap();
            if matches!(ch, 'V' | 'D' | 'I' | 'W' | 'E' | 'F')
                && let Some(pos) = line.find(&format!(" {} ", ch)) {
                    let after_level = &line[pos + 3..]; // skip " " + level + " "
                    return Some((ch, after_level));
                }
        }
    }
    None
}

fn wait_for_pid_via_ps(adb_path: &Path, package: &str, timeout: Duration) -> Result<String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let output = Command::new(adb_path)
            .args(["shell", "ps", "-A", "-o", "PID,NAME"])
            .output()
            .context("Failed to run ps on device")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == package {
                return Ok(parts[0].to_string());
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    bail!(
        "Process with package '{}' not found within {:?}",
        package,
        timeout
    )
}

fn stream_android_logs(crate_name: &str) -> Result<()> {
    use colored::*;

    let adb_path = find_tool("adb")?;
    let package_name = format!("com.goyda.{}", crate_name);

    let pid = wait_for_pid_via_ps(&adb_path, &package_name, Duration::from_secs(5))?;

    let _ = Command::new(&adb_path).args(["logcat", "-c"]).status();

    let mut child = Command::new(&adb_path)
        .args(["logcat", "-v", "time", "--pid", &pid])
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn logcat")?;

    let stdout = child
        .stdout
        .take()
        .expect("Failed to capture stdout from logcat");

    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Warning: error reading logcat output: {}", e);
                break;
            }
        };

        if line.starts_with("---------") {
            continue;
        }

        if let Some((level, _rest)) = parse_log_level(&line) {
            let colored_line = match level {
                'V' => line.truecolor(128, 128, 128),
                'D' => line.blue(),
                'I' => line.green(),
                'W' => line.yellow(),
                'E' => line.red().bold(),
                'F' => line.red().on_red().bold(),
                _ => line.normal(),
            };
            println!("{}", colored_line);
        } else {
            println!("{}", line);
        }
    }

    let _ = child.wait();
    Ok(())
}
