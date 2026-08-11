mod compile;
mod run;

use crate::cli::{Cli, Cmd};
use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.cmd {
        Cmd::Compile { platform, target, manifest_dir } => {
            let platform_str = platform.as_str().to_string();
            let target_str = target.unwrap_or_else(|| {
                platform.handler().supported_triples()[0].to_string()
            });

            compile::compile(platform_str, target_str, manifest_dir)
        }
        Cmd::Run { platform, target, manifest_dir } => {
            let opts = run::RunOptions {
                platform,
                target_triple: target,
                manifest_dir,
            };
            run::execute_run(opts)
        }
    }
}