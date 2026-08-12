use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct WindowsAppLayout {
    runner_dir: PathBuf,
    final_exe: PathBuf,
    assets_dir: PathBuf,
}

impl WindowsAppLayout {
    pub fn new(manifest_dir: &Path, crate_name: &str) -> Self {
        let build_dir = manifest_dir.join("target").join("goyda_windows");
        Self {
            runner_dir: build_dir.join("runner"),
            final_exe: build_dir.join(format!("{crate_name}.exe")),
            assets_dir: build_dir.join("assets"),
        }
    }

    pub fn init_directories(&self) -> Result<()> {
        fs::create_dir_all(self.runner_dir.join("src"))?;
        Ok(())
    }

    pub fn runner_dir(&self) -> &Path {
        &self.runner_dir
    }
    pub fn final_exe(&self) -> &Path {
        &self.final_exe
    }
    pub fn assets_dir(&self) -> &Path {
        &self.assets_dir
    }
}
