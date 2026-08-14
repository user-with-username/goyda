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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn new_parses_name_and_defaults() {
        let cli = Cli::try_parse_from(["goy", "new", "myapp"]).unwrap();
        match cli.cmd {
            Cmd::New {
                name,
                template,
                manifest_dir,
            } => {
                assert_eq!(name, "myapp");
                assert_eq!(template, "counter");
                assert_eq!(manifest_dir, PathBuf::from("."));
            }
            _ => panic!("expected Cmd::New"),
        }
    }

    #[test]
    fn new_accepts_explicit_template_and_manifest_dir() {
        let cli = Cli::try_parse_from([
            "goy",
            "new",
            "myapp",
            "--template",
            "blank",
            "--manifest-dir",
            "/tmp/x",
        ])
        .unwrap();
        match cli.cmd {
            Cmd::New {
                template,
                manifest_dir,
                ..
            } => {
                assert_eq!(template, "blank");
                assert_eq!(manifest_dir, PathBuf::from("/tmp/x"));
            }
            _ => panic!("expected Cmd::New"),
        }
    }

    #[test]
    fn new_requires_a_name() {
        assert!(Cli::try_parse_from(["goy", "new"]).is_err());
    }

    #[test]
    fn init_defaults_template_and_manifest_dir() {
        let cli = Cli::try_parse_from(["goy", "init"]).unwrap();
        match cli.cmd {
            Cmd::Init {
                template,
                manifest_dir,
            } => {
                assert_eq!(template, "counter");
                assert_eq!(manifest_dir, PathBuf::from("."));
            }
            _ => panic!("expected Cmd::Init"),
        }
    }

    #[test]
    fn compile_platform_is_optional() {
        let cli = Cli::try_parse_from(["goy", "compile"]).unwrap();
        match cli.cmd {
            Cmd::Compile {
                platform, release, ..
            } => {
                assert_eq!(platform, None);
                assert!(!release);
            }
            _ => panic!("expected Cmd::Compile"),
        }
    }

    #[test]
    fn compile_parses_a_known_platform() {
        let cli = Cli::try_parse_from(["goy", "compile", "windows"]).unwrap();
        match cli.cmd {
            Cmd::Compile { platform, .. } => assert_eq!(platform, Some(Platform::Windows)),
            _ => panic!("expected Cmd::Compile"),
        }
    }

    #[test]
    fn compile_rejects_an_unknown_platform() {
        assert!(Cli::try_parse_from(["goy", "compile", "nope"]).is_err());
    }

    #[test]
    fn compile_release_flag() {
        let cli = Cli::try_parse_from(["goy", "compile", "--release"]).unwrap();
        match cli.cmd {
            Cmd::Compile { release, .. } => assert!(release),
            _ => panic!("expected Cmd::Compile"),
        }
    }

    #[test]
    fn run_requires_a_platform() {
        assert!(Cli::try_parse_from(["goy", "run"]).is_err());
        let cli = Cli::try_parse_from(["goy", "run", "android"]).unwrap();
        match cli.cmd {
            Cmd::Run {
                platform,
                target,
                release,
                ..
            } => {
                assert_eq!(platform, Platform::Android);
                assert_eq!(target, None);
                assert!(!release);
            }
            _ => panic!("expected Cmd::Run"),
        }
    }

    #[test]
    fn run_accepts_an_explicit_target_triple() {
        let cli =
            Cli::try_parse_from(["goy", "run", "android", "--target", "aarch64-linux-android"])
                .unwrap();
        match cli.cmd {
            Cmd::Run { target, .. } => assert_eq!(target.as_deref(), Some("aarch64-linux-android")),
            _ => panic!("expected Cmd::Run"),
        }
    }
}
