use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

pub fn download_file(url: &str, destination: &Path) -> Result<()> {
    let response = ureq::get(url)
        .call()
        .context("Network error while downloading android.jar")?;
    if response.status() != 200 {
        anyhow::bail!("Server returned status {} for URL: {}", response.status(), url);
    }
    let mut file = fs::File::create(destination).context("Failed to create the file on disk")?;
    std::io::copy(&mut response.into_reader(), &mut file)
        .context("Failed to write the downloaded stream to the file")?;
    Ok(())
}

pub fn run_command(command: &mut Command, error_msg: &'static str) -> Result<()> {
    let status = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context(error_msg)?;

    if !status.success() {
        anyhow::bail!("{}", error_msg);
    }
    Ok(())
}

/// Like [`run_command`], but captures the child's output instead of
/// streaming it live - only printed (as part of the error) if the command
/// fails. Used for `cargo build` invocations so a spinner step can own the
/// line instead of cargo's own progress output interleaving with it.
pub fn run_command_quiet(command: &mut Command, error_msg: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("{error_msg}: failed to spawn process"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let log = if !stderr.trim().is_empty() { stderr.into_owned() } else { stdout.into_owned() };
        anyhow::bail!("{error_msg}\n\n{}", log.trim_end());
    }

    Ok(())
}

/// Copies `src`'s contents into `dst` (created if missing), recursing into
/// subdirectories. Used to stage a project's `assets/` directory into each
/// platform's build output - a no-op (`Ok(())`) if `src` doesn't exist.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }

    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory {:?}", dst))?;

    for entry in fs::read_dir(src).with_context(|| format!("Failed to read directory {:?}", src))? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("Failed to copy {:?} to {:?}", src_path, dst_path))?;
        }
    }

    Ok(())
}

pub fn collect_classes(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    collect_classes_rec(dir, &mut files)?;
    Ok(files)
}

fn collect_classes_rec(dir: &Path, files: &mut Vec<String>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_classes_rec(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "class") {
                files.push(path.to_str().context("Invalid class path")?.to_string());
            }
        }
    }
    Ok(())
}
