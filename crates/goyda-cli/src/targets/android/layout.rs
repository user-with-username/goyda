use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AndroidAppLayout {
    build_dir: PathBuf,
    app_dir: PathBuf,
    classes_dir: PathBuf,
    jni_libs_dir: PathBuf,
    assets_dir: PathBuf,
    manifest: PathBuf,
    java_src_dir: PathBuf,
    gen_apk: PathBuf,
    aligned_apk: PathBuf,
    final_apk: PathBuf,
    pub package_name: String,
}

impl AndroidAppLayout {
    pub fn new(manifest_dir: &Path, target: &str, crate_name: &str) -> Result<Self> {
        let build_dir = manifest_dir.join("target").join("goyda_android");
        let app_dir = build_dir.join("app");

        let safe_crate_id = crate_name.replace('-', "_");
        let package_name = format!("com.goyda.{}", safe_crate_id);

        let class_dir_relative = format!("com/goyda/{}", safe_crate_id);
        let mut java_src_dir = app_dir.join("src");
        for part in class_dir_relative.split('/') {
            java_src_dir = java_src_dir.join(part);
        }

        let abi = match target {
            "aarch64-linux-android" => "arm64-v8a",
            "armv7-linux-androideabi" => "armeabi-v7a",
            "i686-linux-android" => "x86",
            "x86_64-linux-android" => "x86_64",
            _ => anyhow::bail!("Unsupported Android target: {}", target),
        };

        Ok(Self {
            classes_dir: build_dir.join("classes"),
            jni_libs_dir: app_dir.join("lib").join(abi),
            assets_dir: app_dir.join("assets"),
            manifest: app_dir.join("AndroidManifest.xml"),
            gen_apk: build_dir.join("app.apk"),
            aligned_apk: build_dir.join("app-aligned.apk"),
            final_apk: build_dir.join(format!("{}.apk", crate_name)),
            build_dir,
            app_dir,
            java_src_dir,
            package_name,
        })
    }

    pub fn init_directories(&self) -> Result<()> {
        if self.build_dir.exists() {
            fs::remove_dir_all(&self.build_dir)?;
        }
        fs::create_dir_all(&self.jni_libs_dir)?;
        fs::create_dir_all(&self.classes_dir)?;
        fs::create_dir_all(&self.java_src_dir)?;
        Ok(())
    }

    pub fn classes_dir(&self) -> &Path {
        &self.classes_dir
    }
    pub fn jni_libs_dir(&self) -> &Path {
        &self.jni_libs_dir
    }
    pub fn assets_dir(&self) -> &Path {
        &self.assets_dir
    }
    pub fn manifest(&self) -> &Path {
        &self.manifest
    }
    pub fn java_src_dir(&self) -> &Path {
        &self.java_src_dir
    }
    pub fn build_dir(&self) -> &Path {
        &self.build_dir
    }
    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }
    pub fn gen_apk(&self) -> &Path {
        &self.gen_apk
    }
    pub fn aligned_apk(&self) -> &Path {
        &self.aligned_apk
    }
    pub fn final_apk(&self) -> &Path {
        &self.final_apk
    }
}
