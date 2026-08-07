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
    Compile {
        #[arg(long, default_value = "android", value_parser = parse_platform)]
        platform: Platform,

        #[arg(long)]
        target: Option<String>,

        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,
    },

    Run {
        #[arg(long, default_value = "android", value_parser = parse_platform)]
        platform: Platform,

        #[arg(long)]
        target: Option<String>,

        #[arg(long, default_value = ".")]
        manifest_dir: PathBuf,
    },
}

fn parse_platform(s: &str) -> Result<Platform, String> {
    Platform::from_str(s).map_err(|e| e.to_string())
}