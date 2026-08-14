use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::term;

const CARGO_TOML_TEMPLATE: &str = include_str!("../../templates/new/Cargo.toml.template");
const COUNTER_LIB_TEMPLATE: &str = include_str!("../../templates/new/counter.rs");
const BLANK_LIB_TEMPLATE: &str = include_str!("../../templates/new/blank.rs");

const AVAILABLE_TEMPLATES: &[&str] = &["counter", "blank"];

fn lib_template(name: &str) -> Result<&'static str> {
    match name {
        "counter" => Ok(COUNTER_LIB_TEMPLATE),
        "blank" => Ok(BLANK_LIB_TEMPLATE),
        other => bail!(
            "Unknown template '{other}'. Available templates: {}",
            AVAILABLE_TEMPLATES.join(", ")
        ),
    }
}

fn sanitize_package_name(name: &str) -> String {
    let sanitized: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect();

    match sanitized.chars().next() {
        Some(c) if c.is_ascii_alphabetic() => sanitized,
        _ => format!("app-{sanitized}"),
    }
}

fn scaffold(target_dir: &Path, package_name: &str, template: &str) -> Result<()> {
    let lib_src = lib_template(template)?;

    fs::create_dir_all(target_dir.join("src"))
        .with_context(|| format!("Failed to create {:?}", target_dir.join("src")))?;

    let cargo_toml = CARGO_TOML_TEMPLATE.replace("{{package_name}}", package_name);

    fs::write(target_dir.join("Cargo.toml"), cargo_toml).context("Failed to write Cargo.toml")?;
    fs::write(target_dir.join("src").join("lib.rs"), lib_src).context("Failed to write src/lib.rs")?;

    Ok(())
}

pub fn new(name: String, template: String, manifest_dir: PathBuf) -> Result<()> {
    if name.trim().is_empty() {
        bail!("Project name cannot be empty");
    }

    let target_dir = manifest_dir.join(&name);
    if target_dir.exists() {
        bail!("'{}' already exists", target_dir.display());
    }

    let package_name = sanitize_package_name(&name);
    scaffold(&target_dir, &package_name, &template)?;

    term::header("new", &[&package_name, &template]);
    term::info(&format!("Created {}", target_dir.display()));
    term::info(&format!("cd {name} && goy run windows"));

    Ok(())
}

pub fn init(template: String, manifest_dir: PathBuf) -> Result<()> {
    if manifest_dir.join("Cargo.toml").exists() {
        bail!("Cargo.toml already exists in {:?}", manifest_dir);
    }

    fs::create_dir_all(&manifest_dir)
        .with_context(|| format!("Failed to create {:?}", manifest_dir))?;

    let canonical_dir = dunce::canonicalize(&manifest_dir).unwrap_or_else(|_| manifest_dir.clone());
    let dir_name = canonical_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app");
    let package_name = sanitize_package_name(dir_name);

    scaffold(&manifest_dir, &package_name, &template)?;

    term::header("init", &[&package_name, &template]);
    term::info("Created Cargo.toml and src/lib.rs");
    term::info("goy run windows");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A fresh, empty directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("goyda_cli_test_{}_{n}", std::process::id()));
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
    fn sanitize_package_name_lowercases_and_replaces_invalid_chars() {
        assert_eq!(sanitize_package_name("My App"), "my-app");
        assert_eq!(sanitize_package_name("Foo_Bar-2"), "foo_bar-2");
        assert_eq!(sanitize_package_name("  trimmed  "), "trimmed");
    }

    #[test]
    fn sanitize_package_name_prefixes_names_that_would_start_with_a_non_letter() {
        assert_eq!(sanitize_package_name("123"), "app-123");
        assert_eq!(sanitize_package_name(""), "app-");
        assert_eq!(sanitize_package_name("-foo"), "app--foo");
    }

    #[test]
    fn lib_template_resolves_known_names() {
        assert_eq!(lib_template("counter").unwrap(), COUNTER_LIB_TEMPLATE);
        assert_eq!(lib_template("blank").unwrap(), BLANK_LIB_TEMPLATE);
    }

    #[test]
    fn lib_template_rejects_unknown_names() {
        assert!(lib_template("nope").is_err());
    }

    #[test]
    fn scaffold_writes_cargo_toml_and_lib_rs() {
        let dir = TempDir::new();
        scaffold(&dir.0, "my-pkg", "blank").unwrap();

        let cargo_toml = fs::read_to_string(dir.0.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"my-pkg\""));
        assert!(!cargo_toml.contains("{{package_name}}"));

        let lib_rs = fs::read_to_string(dir.0.join("src").join("lib.rs")).unwrap();
        assert_eq!(lib_rs, BLANK_LIB_TEMPLATE);
    }

    #[test]
    fn scaffold_rejects_unknown_template_without_writing_anything() {
        let dir = TempDir::new();
        assert!(scaffold(&dir.0, "my-pkg", "nope").is_err());
        assert!(!dir.0.join("Cargo.toml").exists());
    }

    #[test]
    fn new_creates_a_subdirectory_named_after_the_project() {
        let parent = TempDir::new();
        new("my-app".to_string(), "counter".to_string(), parent.0.clone()).unwrap();

        let project_dir = parent.0.join("my-app");
        assert!(project_dir.join("Cargo.toml").exists());
        assert!(project_dir.join("src").join("lib.rs").exists());
    }

    #[test]
    fn new_rejects_an_empty_name() {
        let parent = TempDir::new();
        assert!(new("  ".to_string(), "counter".to_string(), parent.0.clone()).is_err());
    }

    #[test]
    fn new_rejects_an_already_existing_directory() {
        let parent = TempDir::new();
        fs::create_dir(parent.0.join("taken")).unwrap();
        assert!(new("taken".to_string(), "counter".to_string(), parent.0.clone()).is_err());
    }

    #[test]
    fn init_writes_into_the_given_directory_directly() {
        let dir = TempDir::new();
        init("blank".to_string(), dir.0.clone()).unwrap();

        assert!(dir.0.join("Cargo.toml").exists());
        assert!(dir.0.join("src").join("lib.rs").exists());
    }

    #[test]
    fn init_rejects_a_directory_that_already_has_a_cargo_toml() {
        let dir = TempDir::new();
        fs::write(dir.0.join("Cargo.toml"), "[package]").unwrap();
        assert!(init("counter".to_string(), dir.0.clone()).is_err());
    }
}
