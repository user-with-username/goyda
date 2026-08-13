mod compile;
mod run;

use crate::cli::{Cli, Cmd};
use crate::targets::Platform;
use anyhow::{bail, Result};

pub fn run(cli: Cli) -> Result<()> {
    match cli.cmd {
        Cmd::Compile { platform, target, manifest_dir, release } => match platform {
            Some(platform) => {
                let platform_str = platform.as_str().to_string();
                let target_str = target.unwrap_or_else(|| {
                    platform.handler().supported_triples()[0].to_string()
                });

                compile::compile(platform_str, target_str, manifest_dir, release)
            }
            None => {
                if target.is_some() {
                    bail!("--target requires a platform to be specified");
                }

                for platform in Platform::all().iter().filter(|p| p.handler().is_implemented()) {
                    let platform_str = platform.as_str().to_string();
                    let target_str = platform.handler().supported_triples()[0].to_string();
                    compile::compile(platform_str, target_str, manifest_dir.clone(), release)?;
                }

                Ok(())
            }
        },
        Cmd::Run { platform, target, manifest_dir, release } => {
            let opts = run::RunOptions {
                platform,
                target_triple: target,
                manifest_dir,
                release,
            };
            run::execute_run(opts)
        }
    }
}