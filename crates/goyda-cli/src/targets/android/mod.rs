use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::time::{Duration, Instant};
use std::thread;

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

        let s = term::spinner_step("dev toolchain");
        let env = match BuildEnvironment::prepare() {
            Ok(env) => {
                s.ok();
                env
            }
            Err(e) => {
                s.fail(&e.to_string());
                return Err(e);
            }
        };

        let layout = AndroidAppLayout::new(&ctx.manifest_dir, &ctx.target_triple, &ctx.crate_name)?;
        layout.init_directories()?;

        let s = term::spinner_step("rust -> android");
        let result = native::build_native_library(ctx).and_then(|_| native::copy_native_library(ctx, &layout));
        match result {
            Ok(()) => s.ok(),
            Err(e) => {
                s.fail(&e.to_string());
                return Err(e);
            }
        }

        let s = term::step("layout & assets");
        templates::generate_assets_from_templates(&layout, &ctx.crate_name, &ctx.lib_name)?;
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

    fn stream_logs(&self, ctx: &BuildContext, start: Instant) -> Result<()> {
        stream_android_logs(&ctx.crate_name, start)
    }
}

fn run_adb_quiet(adb_path: &Path, args: &[&str], context_msg: &str) -> Result<()> {
    let output = Command::new(adb_path)
        .args(args)
        .output()
        .with_context(|| format!("{context_msg}: failed to spawn adb"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() { stderr } else { stdout };
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

    let apk_path = artifact_path.to_str().context("Invalid APK path")?;
    run_adb_quiet(
        adb_path,
        &["install", "-r", apk_path],
        "adb install failed",
    )?;

    let package_name = format!("com.goyda.{}", ctx.crate_name);
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
            if matches!(ch, 'V' | 'D' | 'I' | 'W' | 'E' | 'F') {
                if let Some(pos) = line.find(&format!(" {} ", ch)) {
                    let after_level = &line[pos + 3..]; // skip " " + level + " "
                    return Some((ch, after_level));
                }
            }
        }
    }
    None
}

fn wait_for_pid_via_ps(adb_path: &Path, package: &str, timeout: Duration) -> Result<String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let output = Command::new(adb_path)
            .args(&["shell", "ps", "-A", "-o", "PID,NAME"])
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

fn stream_android_logs(crate_name: &str, start: Instant) -> Result<()> {
    use colored::*;

    let adb_path = find_tool("adb")?;
    let package_name = format!("com.goyda.{}", crate_name);

    let pid = wait_for_pid_via_ps(&adb_path, &package_name, Duration::from_secs(5))?;

    term::ready("", start.elapsed());

    let _ = Command::new(&adb_path)
        .args(&["logcat", "-c"])
        .status();

    let mut child = Command::new(&adb_path)
        .args(&["logcat", "-v", "time", "--pid", &pid])
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
                _   => line.normal(),
            };
            println!("{}", colored_line);
        } else {
            println!("{}", line);
        }
    }

    let _ = child.wait();
    Ok(())
}