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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("goyda_cli_fs_test_{}_{n}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn run_command_succeeds_on_a_zero_exit_status() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "exit", "0"]);
        assert!(run_command(&mut cmd, "should not fail").is_ok());
    }

    #[test]
    fn run_command_errors_on_a_nonzero_exit_status() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "exit", "1"]);
        assert!(run_command(&mut cmd, "expected failure").is_err());
    }

    #[test]
    fn run_command_quiet_succeeds_on_a_zero_exit_status() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "exit", "0"]);
        assert!(run_command_quiet(&mut cmd, "should not fail").is_ok());
    }

    #[test]
    fn run_command_quiet_error_includes_stderr_output() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "echo something went wrong 1>&2 & exit 1"]);
        let err = run_command_quiet(&mut cmd, "expected failure").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("expected failure"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn copy_dir_recursive_is_a_noop_when_source_is_missing() {
        let dst = TempDir::new();
        let missing_src = dst.0.join("does-not-exist");
        assert!(copy_dir_recursive(&missing_src, &dst.0.join("out")).is_ok());
        assert!(!dst.0.join("out").exists());
    }

    #[test]
    fn copy_dir_recursive_copies_files_and_nested_directories() {
        let src = TempDir::new();
        let dst = TempDir::new();

        fs::write(src.0.join("a.txt"), "a").unwrap();
        fs::create_dir(src.0.join("nested")).unwrap();
        fs::write(src.0.join("nested").join("b.txt"), "b").unwrap();

        copy_dir_recursive(&src.0, &dst.0).unwrap();

        assert_eq!(fs::read_to_string(dst.0.join("a.txt")).unwrap(), "a");
        assert_eq!(fs::read_to_string(dst.0.join("nested").join("b.txt")).unwrap(), "b");
    }

    #[test]
    fn collect_classes_finds_class_files_recursively_and_ignores_others() {
        let dir = TempDir::new();
        fs::write(dir.0.join("A.class"), "").unwrap();
        fs::write(dir.0.join("readme.txt"), "").unwrap();
        fs::create_dir(dir.0.join("pkg")).unwrap();
        fs::write(dir.0.join("pkg").join("B.class"), "").unwrap();

        let mut classes = collect_classes(&dir.0).unwrap();
        classes.sort();

        assert_eq!(classes.len(), 2);
        assert!(classes[0].ends_with("A.class"));
        assert!(classes[1].ends_with("B.class"));
    }

    #[test]
    fn collect_classes_on_a_missing_directory_returns_empty() {
        let dir = TempDir::new();
        let classes = collect_classes(&dir.0.join("nope")).unwrap();
        assert!(classes.is_empty());
    }
}
