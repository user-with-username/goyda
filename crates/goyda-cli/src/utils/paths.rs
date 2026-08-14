use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use which::which;

pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let mut s = if path.exists() {
        let canonical = dunce::canonicalize(path).context("Failed to canonicalize path")?;
        canonical
            .to_str()
            .context("Invalid UTF-8 in path")?
            .to_string()
    } else {
        path.to_str().context("Invalid UTF-8 in path")?.to_string()
    };
    if s.starts_with(r"\\?\") {
        s = s[4..].to_string();
    }
    Ok(s)
}

pub fn find_tool(name: &str) -> Result<PathBuf> {
    if let Ok(path) = which(name) {
        return Ok(path);
    }
    #[cfg(windows)]
    if let Ok(path) = which(&format!("{}.exe", name)) {
        return Ok(path);
    }
    let android_home = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .ok();
    if let Some(home) = android_home {
        let build_tools_dir = Path::new(&home).join("build-tools");
        if build_tools_dir.exists() {
            if let Ok(entries) = fs::read_dir(&build_tools_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let version_dir = entry.path();
                    if version_dir.is_dir() {
                        #[cfg(windows)]
                        let tool_path = version_dir.join(&format!("{}.exe", name));
                        if tool_path.exists() {
                            return Ok(tool_path);
                        }
                    }
                }
            }
        }
    }
    anyhow::bail!(
        "{} not found. Make sure the Android SDK build-tools are on PATH, or set ANDROID_HOME",
        name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_returns_a_nonexistent_path_unchanged() {
        let result = normalize_path("this/path/does/not/exist/anywhere").unwrap();
        assert_eq!(result, "this/path/does/not/exist/anywhere");
    }

    #[test]
    fn normalize_path_canonicalizes_an_existing_path_without_a_verbatim_prefix() {
        let dir = std::env::temp_dir();
        let result = normalize_path(&dir).unwrap();
        assert!(!result.starts_with(r"\\?\"), "got: {result}");
        // The canonicalized path must still point at the same directory.
        assert!(Path::new(&result).exists());
    }
}