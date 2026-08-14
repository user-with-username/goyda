use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use crate::targets::{BuildContext, Platform, PlatformTarget};
use crate::term;

mod build;
mod layout;

use layout::WindowsAppLayout;

pub struct WindowsTarget;

impl PlatformTarget for WindowsTarget {
    fn id(&self) -> Platform {
        Platform::Windows
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &[
            "x86_64-pc-windows-msvc",
            "i686-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
        ]
    }

    fn build(&self, ctx: &BuildContext) -> Result<PathBuf> {
        term::header("UI", &[&ctx.target_triple]);
        build::build_windows_app(ctx)
    }

    /// Launches `bin/{crate_name}.exe` (the persistent host - see
    /// `build.rs`'s doc comments) pointed at whatever the *highest*
    /// generation dylib already sitting in `bin/` is, and streams its
    /// stdout/stderr into this terminal on background threads, returning as
    /// soon as the process is spawned - `run.rs`'s hot-reload loop needs its
    /// main thread free to listen for the next `r`/`R`/`q` keypress right
    /// after this returns, not blocked waiting for the app to exit. Any
    /// previous instance (from an earlier reload) is asked to close first -
    /// see [`close_previous_instance`].
    fn run(&self, ctx: &BuildContext, artifact_path: &Path) -> Result<()> {
        close_previous_instance();

        let layout = WindowsAppLayout::new(&ctx.manifest_dir, &ctx.crate_name);
        let dylib_path = latest_generation_dylib(&layout)
            .context("No consumer dylib generation found in bin/ - build() should have produced app_gen0.dll")?;

        let mut child = Command::new(artifact_path)
            .arg(&dylib_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to launch the built .exe")?;

        let stdout = child
            .stdout
            .take()
            .expect("child spawned with piped stdout");
        let stderr = child
            .stderr
            .take()
            .expect("child spawned with piped stderr");

        thread::spawn(move || stream_lines(stdout, false));
        thread::spawn(move || stream_lines(stderr, true));

        // Deliberately not `.wait()`-ed: dropping a `Child` doesn't kill it
        // on any platform, so the app keeps running as its own detached
        // process - exactly what lets this `run()` return immediately.
        drop(child);
        Ok(())
    }

    /// The real hot-reload path: if the host is already running (found via
    /// its `GoydaRoot` window class), this rebuilds *only* the consumer
    /// crate's dylib, drops it into `bin/` under a fresh generation
    /// filename, and hands that path to the running process via
    /// `WM_COPYDATA` - see `goyda::windows::hot_swap_dylib` for the
    /// receiving half. No new process, no window/message-loop restart, no
    /// reinstall - the thing the old file-based state-snapshot hack could
    /// never actually avoid. Falls through to the ordinary `build`+`run`
    /// (returns `Ok(false)`) whenever there's nothing running yet to patch,
    /// e.g. the very first `r` right after `goy run windows` starts.
    fn quick_reload(&self, ctx: &BuildContext) -> Result<bool> {
        let Some(hwnd) = find_root_window() else {
            return Ok(false);
        };

        let layout = WindowsAppLayout::new(&ctx.manifest_dir, &ctx.crate_name);
        let built_dll = build::build_consumer_dylib(&layout, ctx)?;

        let generation_path = build::next_generation_path(&layout);
        std::fs::copy(&built_dll, &generation_path)
            .context("Failed to copy the rebuilt dylib into bin/ under its next generation name")?;

        send_hot_swap(hwnd, &generation_path)?;
        Ok(true)
    }

    /// Kills any running instance and clears out every previous
    /// `app_gen*.dll` in `bin/`, so the next `build`+`run` (what `R` falls
    /// through to - it doesn't use [`Self::quick_reload`] at all) starts
    /// clean at generation 0 again instead of the generation counter
    /// climbing forever across full reloads too.
    fn full_reset(&self, ctx: &BuildContext) -> Result<()> {
        close_previous_instance();

        let layout = WindowsAppLayout::new(&ctx.manifest_dir, &ctx.crate_name);
        if let Ok(entries) = std::fs::read_dir(layout.bin_dir()) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("app_gen") && name.ends_with(".dll") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        Ok(())
    }
}

fn latest_generation_dylib(layout: &WindowsAppLayout) -> Option<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    let entries = std::fs::read_dir(layout.bin_dir()).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        if let Some(rest) = name
            .strip_prefix("app_gen")
            .and_then(|r| r.strip_suffix(".dll"))
        {
            if let Ok(n) = rest.parse::<u32>() {
                if best.as_ref().map_or(true, |(m, _)| n > *m) {
                    best = Some((n, entry.path()));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// `"GoydaRoot"` - see `goyda::windows::mod::ROOT_CLASS_NAME`. `None` means
/// no host is currently running (nothing to hot-swap into).
#[cfg(windows)]
fn find_root_window() -> Option<isize> {
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    let class_name: Vec<u16> = "GoydaRoot\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(class_name.as_ptr(), std::ptr::null()) };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd as isize)
    }
}

#[cfg(not(windows))]
fn find_root_window() -> Option<isize> {
    None
}

/// Sends `path` (the freshly built generation's dylib) to the already-
/// running host via `WM_COPYDATA` - the standard way to hand an arbitrary
/// byte buffer to another *process*'s window (plain `WPARAM`/`LPARAM` are
/// just integers, meaningless across a process boundary; `WM_COPYDATA`'s
/// payload is kernel-marshaled into the receiver automatically). See
/// `goyda::windows::mod::root_wndproc`'s `WM_COPYDATA` arm for the other
/// side.
#[cfg(windows)]
fn send_hot_swap(hwnd: isize, path: &Path) -> Result<()> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_COPYDATA};

    let path_str = path
        .to_str()
        .context("windows dylib path is not valid UTF-8")?;
    let bytes = path_str.as_bytes();

    let cds = COPYDATASTRUCT {
        dwData: 0,
        cbData: bytes.len() as u32,
        lpData: bytes.as_ptr() as *mut _,
    };

    unsafe {
        SendMessageW(
            hwnd as HWND,
            WM_COPYDATA,
            0,
            &cds as *const COPYDATASTRUCT as isize,
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn send_hot_swap(_hwnd: isize, _path: &Path) -> Result<()> {
    anyhow::bail!("hot-swap is only implemented on a windows host")
}

/// Asks whichever window carries goyda's root window class (`"GoydaRoot"` -
/// see `goyda::windows::mod::ROOT_CLASS_NAME`) to close, the same
/// `WM_CLOSE` an ordinary user click on the titlebar's X sends, before
/// spawning a freshly built `.exe` (used by [`WindowsTarget::run`] and
/// [`WindowsTarget::full_reset`] - the two paths that actually replace the
/// running process, unlike [`WindowsTarget::quick_reload`] which never
/// touches it). A no-op on non-Windows hosts (nothing to find/close).
#[cfg(windows)]
fn close_previous_instance() {
    use std::time::Duration;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};

    let class_name: Vec<u16> = "GoydaRoot\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(class_name.as_ptr(), std::ptr::null()) };
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        thread::sleep(Duration::from_millis(300));
    }
}

#[cfg(not(windows))]
fn close_previous_instance() {}

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
