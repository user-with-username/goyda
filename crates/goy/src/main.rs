mod cli;
mod commands;
mod constants;
mod targets;
mod term;
mod utils;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    commands::run(cli)
}
