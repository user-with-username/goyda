use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{AndroidAppLayout, BuildEnvironment};
use crate::utils::{collect_classes, find_tool, normalize_path, run_command};

pub fn build_apk_package(layout: &AndroidAppLayout, env: &BuildEnvironment) -> Result<()> {
    let aapt2_path = find_tool("aapt2")?;
    let javac_path = find_tool("javac")?;
    let d8_path = find_tool("d8")?;
    let zipalign_path = find_tool("zipalign")?;
    let apksigner_path = find_tool("apksigner")?;

    let android_jar = &env.android_jar;
    let manifest = layout.manifest();
    let classes_dir = layout.classes_dir();
    let build_dir = layout.build_dir();
    
    let apk_path = layout.gen_apk();
    let aligned_apk_path = layout.aligned_apk();
    let final_apk_path = layout.final_apk();
    let keystore_path = &env.debug_keystore;

    let _cleanup = defer_cleanup(vec![apk_path.to_path_buf(), aligned_apk_path.to_path_buf()]);

    run_command(
        Command::new(&aapt2_path).args(&[
            "link",
            "-I", &normalize_path(android_jar)?,
            "--manifest", &normalize_path(&manifest)?,
            "--min-sdk-version", "21",
            "--target-sdk-version", "33",
            "-o", &normalize_path(&apk_path)?,
        ]),
        "Building resources via aapt2 link failed",
    )?;

    let java_sources = collect_java_sources(&layout.java_src_dir())?;
    if java_sources.is_empty() {
        anyhow::bail!("No .java files found in directory {:?}", layout.java_src_dir());
    }

    let mut javac_args = vec![
        "-cp".to_string(),
        normalize_path(android_jar)?,
        "-d".to_string(),
        normalize_path(&classes_dir)?,
        "--release".to_string(),
        "8".to_string(),
    ];
    for source in &java_sources {
        javac_args.push(normalize_path(source)?);
    }

    run_command(
        Command::new(&javac_path).args(&javac_args),
        "Compiling Java sources via javac failed",
    )?;

    let class_files = collect_classes(&classes_dir)?;
    if class_files.is_empty() {
        anyhow::bail!("No compiled .class files found in directory {:?}", classes_dir);
    }

    let normalized_classes: Vec<String> = class_files
        .iter()
        .map(normalize_path)
        .collect::<Result<_>>()?;

    let mut d8_args = vec![
        "--min-api".into(),
        "21".into(),
        "--output".into(),
        normalize_path(&build_dir)?,
        "--lib".into(),
        normalize_path(android_jar)?,
    ];
    d8_args.extend(normalized_classes);

    run_command(
        Command::new(&d8_path).args(&d8_args),
        "Generating dex files via d8 failed",
    )?;

    let dex_path = build_dir.join("classes.dex");
    let lib_dir = layout.app_dir().join("lib");
    
    repack_apk_with_dex_and_libs(&apk_path, &dex_path, &lib_dir)
        .context("Failed to pack dex and native libraries into the APK")?;

    run_command(
        Command::new(&zipalign_path).args(&[
            "-f", "4",
            &normalize_path(&apk_path)?,
            &normalize_path(&aligned_apk_path)?,
        ]),
        "Aligning the package via zipalign failed",
    )?;

    if !aligned_apk_path.exists() {
        anyhow::bail!("File to sign not found: {:?}", aligned_apk_path);
    }

    if let Some(parent) = final_apk_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context("Failed to create the output directory for the final APK")?;
        }
    }

    let key_pass_arg = format!("pass:{}", crate::constants::DEFAULT_KEY_PASS);
    run_command(
        Command::new(&apksigner_path).args(&[
            "sign",
            "--ks", &normalize_path(keystore_path)?,
            "--ks-pass", &key_pass_arg,
            "--out", &normalize_path(&final_apk_path)?,
            &normalize_path(&aligned_apk_path)?,
        ]),
        "Signing the app via apksigner failed",
    )?;

    Ok(())
}

fn repack_apk_with_dex_and_libs(
    apk_path: &Path,
    dex_path: &Path,
    lib_dir: &Path,
) -> Result<()> {
    let temp_apk_path = apk_path.with_extension("tmp.apk");

    let src_file = File::open(apk_path).context("Failed to open source APK")?;
    let mut archive = ZipArchive::new(src_file).context("Failed to parse source APK")?;

    let dst_file = File::create(&temp_apk_path).context("Failed to create temporary APK")?;
    let mut zip = ZipWriter::new(dst_file);

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        zip.raw_copy_file(file)?;
    }

    if dex_path.exists() {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        zip.start_file("classes.dex", options)?;
        let mut dex_bytes = Vec::new();
        File::open(dex_path)?.read_to_end(&mut dex_bytes)?;
        zip.write_all(&dex_bytes)?;
    }

    if lib_dir.exists() && lib_dir.is_dir() {
        add_dir_to_zip(&mut zip, lib_dir, lib_dir, "lib")?;
    }

    zip.finish()?;

    fs::rename(&temp_apk_path, apk_path)?;

    Ok(())
}

fn add_dir_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    current_dir: &Path,
    base_dir: &Path,
    prefix: &str,
) -> Result<()> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored);

    for entry in fs::read_dir(current_dir)? {
        let path = entry?.path();

        if path.is_dir() {
            add_dir_to_zip(zip, &path, base_dir, prefix)?;
        } else {
            let relative_path = path.strip_prefix(base_dir)?;
            let archive_path = Path::new(prefix).join(relative_path);

            let zip_entry_name = archive_path.to_string_lossy().replace('\\', "/");

            zip.start_file(zip_entry_name, options)?;
            let mut file_bytes = Vec::new();
            File::open(&path)?.read_to_end(&mut file_bytes)?;
            zip.write_all(&file_bytes)?;
        }
    }
    Ok(())
}

fn collect_java_sources(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    if dir.exists() && dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                sources.extend(collect_java_sources(&path)?);
            } else if path.extension().and_then(|s| s.to_str()) == Some("java") {
                sources.push(path);
            }
        }
    }
    Ok(sources)
}

struct CleanUpGuard(Vec<PathBuf>);

impl Drop for CleanUpGuard {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn defer_cleanup(paths: Vec<PathBuf>) -> CleanUpGuard {
    CleanUpGuard(paths)
}