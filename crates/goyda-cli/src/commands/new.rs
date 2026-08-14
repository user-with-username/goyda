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
