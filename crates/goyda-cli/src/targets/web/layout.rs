use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct WebAppLayout {
    dist_dir: PathBuf,
}

impl WebAppLayout {
    pub fn new(manifest_dir: &Path) -> Self {
        let dist_dir = manifest_dir.join("target").join("goyda_web").join("dist");
        Self { dist_dir }
    }

    pub fn init_directories(&self) -> Result<()> {
        if self.dist_dir.exists() {
            fs::remove_dir_all(&self.dist_dir)?;
        }
        fs::create_dir_all(&self.dist_dir)?;
        Ok(())
    }

    pub fn dist_dir(&self) -> &Path {
        &self.dist_dir
    }
}
