use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::targets::BuildContext;
use crate::term;
use crate::utils::{copy_dir_recursive, run_command_quiet};

use super::layout::WindowsAppLayout;

/// `-C prefer-dynamic` is the whole trick this build revolves around: by
/// default `cargo build` statically links every dependency (including
/// `goyda`) into whatever it's building, so the host exe and the consumer's
/// `cdylib` would each get their own *private* copy of `goyda`'s statics
/// (window/control state, the `inventory` page registry, ...) - useless for
/// a hot swap, since the host's copy would never see anything the consumer
/// dylib registers. `-C prefer-dynamic` makes both link against `goyda`'s
/// `dylib` output (`goyda.dll`, from `[lib] crate-type = ["rlib", "dylib"]`
/// in `crates/goyda/Cargo.toml`) instead - one shared instance, loaded once,
/// that every generation of the consumer dylib dynamically imports by name
/// (`goyda.dll`) rather than linking in its own copy. See
/// `goyda::windows::hot_swap_dylib`'s doc comment for the mounting half of
/// this, and [`WindowsAppLayout`]'s doc comment for why the host and the
/// consumer dylib *also* have to be built through one single workspace-wide
/// `cargo build`, not two separate ones - having them agree on which
/// `goyda.dll` to dynamically link against isn't enough on its own.
const PREFER_DYNAMIC: &str = "-C prefer-dynamic";

pub fn build_windows_app(ctx: &BuildContext) -> Result<PathBuf> {
    let layout = WindowsAppLayout::new(&ctx.manifest_dir, &ctx.crate_name);
    layout.init_directories()?;

    ensure_target_installed(&ctx.target_triple)?;

    let s = term::step("layout & assets");
    let (goyda_dir, workspace_lockfile) = resolve_goyda_manifest_dir(ctx)?;
    write_workspace(&layout, ctx, &goyda_dir, &workspace_lockfile)?;
    s.ok();

    let s = term::spinner_step("host + native library");
    let result = compile_workspace(&layout, ctx, None);
    match &result {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    result?;

    let s = term::step("project assets");
    copy_dir_recursive(&ctx.manifest_dir.join("assets"), layout.assets_dir())
        .context("Failed to copy the assets/ directory into the windows build")?;
    s.ok();

    let s = term::spinner_step("assemble runtime dir");
    let assembled = assemble_bin_dir(&layout, ctx, true);
    match &assembled {
        Ok(()) => s.ok(),
        Err(e) => s.fail(&e.to_string()),
    }
    assembled?;

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
/// on lives on disk (so the generated workspace - see [`write_workspace`] -
/// can depend on that exact same `goyda` by path instead of guessing a
/// version), plus the consumer's own resolved `Cargo.lock` path - copied
/// into the synthetic workspace so it pins `goyda`'s transitive
/// dependencies (`windows-sys`, `resvg`, `image`, ...) to the exact same
/// versions the consumer already resolved, rather than re-resolving them
/// (possibly differently, e.g. if a new semver-compatible release shipped
/// since) from scratch.
fn resolve_goyda_manifest_dir(ctx: &BuildContext) -> Result<(PathBuf, PathBuf)> {
    let meta = MetadataCommand::new()
        .manifest_path(ctx.manifest_dir.join("Cargo.toml"))
        .exec()
        .context("Failed to resolve the project's dependency graph via `cargo metadata`")?;

    let goyda = meta
        .packages
        .iter()
        .find(|p| p.name == "goyda")
        .context("Could not find `goyda` in the resolved dependency graph")?;

    let goyda_dir = goyda
        .manifest_path
        .parent()
        .context("goyda's manifest path has no parent directory")?
        .as_std_path()
        .to_path_buf();

    let lockfile = meta.workspace_root.as_std_path().join("Cargo.lock");

    Ok((goyda_dir, lockfile))
}

/// Writes the synthetic two-member workspace `compile_workspace` builds -
/// see [`WindowsAppLayout`]'s doc comment for why a real `[workspace]`
/// (versus two independently-invoked `cargo build`s) is the actual point,
/// not just an implementation detail. `host` (see [`write_host_crate`]) is
/// the persistent half; `shim` (see [`write_consumer_shim`]) exists only to
/// get the *consumer's own* crate built as a `cdylib` from inside this
/// workspace (its real manifest lives in the consumer's own project/
/// workspace, which this one never touches or nests inside of - `shim` just
/// takes a normal path dependency on it, same as any other dependency).
fn write_workspace(layout: &WindowsAppLayout, ctx: &BuildContext, goyda_dir: &Path, workspace_lockfile: &Path) -> Result<()> {
    let workspace_toml = r#"[workspace]
resolver = "2"
members = ["host", "shim"]
"#;
    fs::write(layout.workspace_dir().join("Cargo.toml"), workspace_toml)
        .context("Failed to write the windows build workspace's Cargo.toml")?;

    if workspace_lockfile.exists() {
        fs::copy(workspace_lockfile, layout.workspace_dir().join("Cargo.lock"))
            .context("Failed to copy the consumer's Cargo.lock into the windows build workspace")?;
    }

    write_host_crate(layout, goyda_dir)?;
    write_consumer_shim(layout, ctx, goyda_dir)?;

    Ok(())
}

/// The persistent half of the app: owns the window, the message loop, and
/// (via `goyda.dll`, loaded once and never rebuilt) every bit of state a
/// reload must survive. Has no `fn main()`-level knowledge of the consumer
/// crate at all - unlike the old runner (which `extern crate`'d the
/// consumer directly so it'd get statically linked in), the consumer's
/// `#[page(...)]`s are only ever reached by `LoadLibraryW`ing its `cdylib`
/// (built by [`write_consumer_shim`]) at runtime (see
/// `goyda::windows::run`/`hot_swap_dylib`), first at startup (the path is
/// passed as `argv[1]` - see [`assemble_bin_dir`]'s launch-time counterpart
/// in `mod.rs`'s `run`) and again on every `r`.
fn write_host_crate(layout: &WindowsAppLayout, goyda_dir: &Path) -> Result<()> {
    let goyda_dir = normalize(goyda_dir)?;

    let cargo_toml = format!(
        r#"[package]
name = "goyda_windows_host"
version = "0.0.0"
edition = "2021"
publish = false

[[bin]]
name = "goyda_windows_host"
path = "src/main.rs"

[dependencies]
goyda = {{ path = "{goyda_dir}", default-features = false, features = ["windows"] }}

[profile.dev]
codegen-units = 1

[profile.release]
codegen-units = 1
"#,
        goyda_dir = goyda_dir,
    );
    fs::write(layout.host_dir().join("Cargo.toml"), cargo_toml)
        .context("Failed to write the windows host crate's Cargo.toml")?;

    let main_rs = r#"fn main() {
    let initial_dylib = std::env::args().nth(1).map(std::path::PathBuf::from);
    goyda::windows::run(initial_dylib.as_deref());
}
"#;
    fs::write(layout.host_dir().join("src").join("main.rs"), main_rs)
        .context("Failed to write the windows host crate's main.rs")?;

    Ok(())
}

/// A thin `cdylib` wrapper around the consumer's own crate (added as an
/// ordinary path dependency - the consumer's real manifest/workspace is
/// untouched and irrelevant to this one) - purely so the consumer's code
/// can be built as one of *this* synthetic workspace's members, which is
/// what makes it share a single compiled `goyda` unit with [`host`](write_host_crate)
/// (see [`WindowsAppLayout`]'s doc comment). `extern crate {lib_ident} as
/// _consumer_crate;` (rather than an empty `lib.rs`) is required, not just
/// for clarity - an unreferenced path dependency's object code (in
/// particular its `#[page(...)]` factories' `inventory::submit!` ctors)
/// can otherwise be dropped by the linker even though cargo built it,
/// exactly as the old statically-linked runner's own doc comment already
/// noted about the same trick.
fn write_consumer_shim(layout: &WindowsAppLayout, ctx: &BuildContext, goyda_dir: &Path) -> Result<()> {
    let goyda_dir = normalize(goyda_dir)?;
    let manifest_dir = normalize(&ctx.manifest_dir)?;

    let cargo_toml = format!(
        r#"[package]
name = "goyda_windows_consumer_shim"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"

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
    fs::write(layout.shim_dir().join("Cargo.toml"), cargo_toml)
        .context("Failed to write the windows consumer shim's Cargo.toml")?;

    let lib_ident = ctx.lib_name.replace('-', "_");
    let lib_rs = format!("extern crate {lib_ident} as _consumer_crate;\n");
    fs::write(layout.shim_dir().join("src").join("lib.rs"), lib_rs)
        .context("Failed to write the windows consumer shim's lib.rs")?;

    Ok(())
}

/// TOML string escaping doesn't need to special-case backslashes if there
/// simply aren't any - Windows paths work fine with forward slashes, and it
/// sidesteps having to escape the path for a TOML string literal.
fn normalize(path: &Path) -> Result<String> {
    Ok(crate::utils::normalize_path(path)?.replace('\\', "/"))
}

/// Builds the synthetic workspace [`write_workspace`] wrote. `package`
/// selects just one member (`-p`) - used by
/// [`super::WindowsTarget::quick_reload`] to rebuild *only* `shim` (the
/// consumer's code) without touching `host` or recompiling `goyda` at all;
/// `None` (a plain `cargo build --workspace`) builds both, for a full
/// [`build_windows_app`]. Either way this is the *only* place `cargo build`
/// is ever invoked for a windows build - see [`WindowsAppLayout`]'s doc
/// comment for why that single-invocation property is load-bearing, not
/// just tidy.
fn compile_workspace(layout: &WindowsAppLayout, ctx: &BuildContext, package: Option<&str>) -> Result<()> {
    let mut args = vec![
        "build".to_string(),
        "--target".to_string(),
        ctx.target_triple.clone(),
        "--target-dir".to_string(),
        layout.shared_target_dir().to_string_lossy().into_owned(),
    ];
    match package {
        Some(name) => {
            args.push("-p".to_string());
            args.push(name.to_string());
        }
        None => args.push("--workspace".to_string()),
    }
    if ctx.release {
        args.push("--release".to_string());
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(layout.workspace_dir()).args(&args).env("RUSTFLAGS", PREFER_DYNAMIC);
    run_command_quiet(&mut cmd, "building the windows app via cargo build failed")
}

/// Rebuilds just the consumer's code (`shim`, see [`write_consumer_shim`])
/// within the already-written workspace and returns the freshly built
/// `cdylib`'s path - the whole body of
/// [`super::WindowsTarget::quick_reload`]'s rebuild step. `goyda` itself is
/// untouched by this (unchanged inputs -> cargo reuses its already-compiled,
/// already-loaded-by-the-running-host `goyda.dll` from cache), so this is
/// fast and - critically - never produces a `goyda.dll` the running host's
/// already-loaded copy could disagree with.
pub fn build_consumer_dylib(layout: &WindowsAppLayout, ctx: &BuildContext) -> Result<PathBuf> {
    compile_workspace(layout, ctx, Some("goyda_windows_consumer_shim"))?;

    let built_dll = layout
        .shared_target_dir()
        .join(&ctx.target_triple)
        .join(ctx.profile_dir())
        .join("goyda_windows_consumer_shim.dll");

    if !built_dll.exists() {
        anyhow::bail!("Compiled consumer dylib not found at: {:?}", built_dll);
    }
    Ok(built_dll)
}

/// The next `app_gen{N}.dll` filename [`super::WindowsTarget::quick_reload`]
/// (or a fresh [`assemble_bin_dir`]) should write to - one past whatever
/// generation already exists in `bin/`, so a Windows-locked (currently
/// loaded) earlier generation's file is never touched, only ever added
/// alongside.
pub fn next_generation_path(layout: &WindowsAppLayout) -> PathBuf {
    let mut max_seen: Option<u32> = None;
    if let Ok(entries) = fs::read_dir(layout.bin_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix("app_gen").and_then(|r| r.strip_suffix(".dll")) {
                if let Ok(n) = rest.parse::<u32>() {
                    max_seen = Some(max_seen.map_or(n, |m| m.max(n)));
                }
            }
        }
    }
    let next = max_seen.map_or(0, |m| m + 1);
    layout.bin_dir().join(format!("app_gen{next}.dll"))
}

/// Finds `std-*.dll` in the active toolchain's target sysroot - `-C
/// prefer-dynamic` links against it dynamically too (not just `goyda.dll`),
/// so it has to sit next to the exe/dlls same as any other runtime
/// dependency; unlike `goyda.dll`, this one's identical for any build of
/// this same toolchain, so it's only ever copied once per `bin/` dir, not
/// regenerated on every reload.
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
        if name.starts_with("std-") && name.ends_with(".dll") {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

/// Copies everything the running app actually needs at runtime into
/// `bin/`: the host exe, `goyda.dll`, the dynamic `std` runtime, and (only
/// when `include_generation_zero`, i.e. a full [`build_windows_app`], not a
/// [`super::WindowsTarget::quick_reload`] which writes its own generation
/// directly into `bin/` - see [`next_generation_path`]) the consumer's
/// first-generation dylib.
fn assemble_bin_dir(layout: &WindowsAppLayout, ctx: &BuildContext, include_generation_zero: bool) -> Result<()> {
    let host_exe = layout
        .shared_target_dir()
        .join(&ctx.target_triple)
        .join(ctx.profile_dir())
        .join("goyda_windows_host.exe");
    if !host_exe.exists() {
        anyhow::bail!("Compiled host executable not found at: {:?}", host_exe);
    }
    fs::copy(&host_exe, layout.final_exe()).context("Failed to copy the host .exe into the windows build output")?;

    let goyda_dll = layout
        .shared_target_dir()
        .join(&ctx.target_triple)
        .join(ctx.profile_dir())
        .join("goyda.dll");
    if !goyda_dll.exists() {
        anyhow::bail!("Compiled goyda.dll not found at: {:?}", goyda_dll);
    }
    fs::copy(&goyda_dll, layout.bin_dir().join("goyda.dll")).context("Failed to copy goyda.dll into the windows build output")?;

    if let Some(std_dll) = find_std_dylib(&ctx.target_triple)? {
        let dest = layout.bin_dir().join(std_dll.file_name().unwrap());
        if !dest.exists() {
            fs::copy(&std_dll, &dest).context("Failed to copy the dynamic std runtime into the windows build output")?;
        }
    }

    if include_generation_zero {
        let built_dll = layout
            .shared_target_dir()
            .join(&ctx.target_triple)
            .join(ctx.profile_dir())
            .join("goyda_windows_consumer_shim.dll");
        let gen0 = layout.bin_dir().join("app_gen0.dll");
        fs::copy(&built_dll, &gen0).context("Failed to copy the consumer dylib into the windows build output")?;
    }

    Ok(())
}
