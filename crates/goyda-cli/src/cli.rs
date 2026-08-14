use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

use crate::targets::Platform;

#[derive(Parser)]
#[command(name = "goy", version, about = "GoydaUI CLI Compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Scaffold a new project into a new directory.
    New {
        /// Directory (and package) name to create.
        name: String,

        /// Starter template to scaffold from.
        #[arg(long, default_value = "counter")]
        template: String,

        /// Directory the new project directory is created inside of.
        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,
    },

    /// Scaffold a new project into the current (or given) directory.
    Init {
        /// Starter template to scaffold from.
        #[arg(long, default_value = "counter")]
        template: String,

        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,
    },

    /// Build the project. Without a platform, builds all implemented platforms.
    Compile {
        #[arg(value_parser = parse_platform)]
        platform: Option<Platform>,

        #[arg(long)]
        target: Option<String>,

        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,

        /// Optimized build (`cargo build --release`) - the finished
        /// artifact also gets copied into `<manifest_dir>/release/<platform>/`
        /// for easy grabbing, alongside its usual `target/` location.
        #[arg(long)]
        release: bool,
    },

    /// Build and run the project for a platform.
    Run {
        #[arg(value_parser = parse_platform)]
        platform: Platform,

        #[arg(long)]
        target: Option<String>,

        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,

        /// Optimized build (`cargo build --release`) - slower to rebuild on
        /// each hot-reload, but useful for checking release-only behavior.
        #[arg(long)]
        release: bool,
    },
}

fn parse_platform(s: &str) -> Result<Platform, String> {
    Platform::from_str(s).map_err(|e| e.to_string())
}