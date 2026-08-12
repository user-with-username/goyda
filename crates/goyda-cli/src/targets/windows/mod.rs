use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use crate::targets::{BuildContext, Platform, PlatformTarget};
use crate::term;

mod build;
mod layout;

pub struct WindowsTarget;

impl PlatformTarget for WindowsTarget {
    fn id(&self) -> Platform {
        Platform::Windows
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &["x86_64-pc-windows-msvc", "i686-pc-windows-msvc", "aarch64-pc-windows-msvc"]
    }

    fn build(&self, ctx: &BuildContext) -> Result<PathBuf> {
        term::header("UI", &[&ctx.target_triple]);
        build::build_windows_app(ctx)
    }

    /// Launches the built `.exe` and streams its stdout/stderr straight into
    /// this terminal until it exits - the windows equivalent of android's
    /// `adb logcat` streaming, just simpler: since this process is the one
    /// that spawned the app (unlike an app running on a separate device),
    /// its output can be piped and read directly instead of needing a
    /// separate log-collection step.
    fn run(&self, ctx: &BuildContext, artifact_path: &Path) -> Result<()> {
        let mut child = Command::new(artifact_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to launch the built .exe")?;

        term::ready("", ctx.start.elapsed());

        let stdout = child.stdout.take().expect("child spawned with piped stdout");
        let stderr = child.stderr.take().expect("child spawned with piped stderr");

        let stdout_thread = thread::spawn(move || stream_lines(stdout, false));
        let stderr_thread = thread::spawn(move || stream_lines(stderr, true));

        let status = child.wait().context("Failed while waiting for the .exe to exit")?;
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        if !status.success() {
            anyhow::bail!("the app exited with a non-zero status: {status}");
        }
        Ok(())
    }
}

fn stream_lines<R: Read>(reader: R, is_stderr: bool) {
    use colored::*;

    for line in BufReader::new(reader).lines() {
        let Ok(line) = line else { break };
        if is_stderr {
            println!("{}", line.red());
        } else {
            println!("{line}");
        }
    }
}
