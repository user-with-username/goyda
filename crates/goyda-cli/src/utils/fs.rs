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
