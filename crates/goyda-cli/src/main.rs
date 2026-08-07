mod cli;
mod commands;
mod utils;
mod constants;
mod targets;
mod term;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    commands::run(cli)
}