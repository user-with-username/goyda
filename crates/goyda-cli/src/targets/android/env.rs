use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::constants::{ANDROID_JAR_URL, DEFAULT_KEY_PASS, GOYDA_DIR_NAME};
use crate::utils::{download_file, run_command};

pub struct BuildEnvironment {
    pub android_jar: PathBuf,
    pub debug_keystore: PathBuf,
}

impl BuildEnvironment {
    pub fn prepare() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not locate the user's home directory")?;
        let goyda_home = home_dir.join(GOYDA_DIR_NAME);
        let cache_dir = goyda_home.join("cache");

        fs::create_dir_all(&cache_dir).context("Failed to create the global cache directory")?;

        let android_jar = cache_dir.join("android.jar");
        let debug_keystore = goyda_home.join("debug.keystore");

        if !android_jar.exists() {
            println!("--- Cache miss: downloading official android.jar ---");
            download_file(ANDROID_JAR_URL, &android_jar)?;
        }

        if !debug_keystore.exists() {
            println!("--- Generating unique local debug.keystore ---");
            run_command(
                Command::new("keytool").args(&[
                    "-genkey",
                    "-v",
                    "-keystore",
                    debug_keystore.to_str().context("Invalid keystore path")?,
                    "-storepass",
                    DEFAULT_KEY_PASS,
                    "-alias",
                    "androiddebugkey",
                    "-keypass",
                    DEFAULT_KEY_PASS,
                    "-keyalg",
                    "RSA",
                    "-keysize",
                    "2048",
                    "-validity",
                    "10000",
                    "-dname",
                    "CN=Goyda Developer,O=GoydaUI,C=RU",
                ]),
                "Generating debug.keystore via keytool failed",
            )?;
        }

        Ok(BuildEnvironment {
            android_jar,
            debug_keystore,
        })
    }
}
