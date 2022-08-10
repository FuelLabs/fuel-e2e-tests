mod commands;
pub mod utils;

use crate::commands::Commands;
use clap::Parser;
use commands::{dispatch_command, Cli};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    dispatch_command(&cli.command).await?;

    Ok(())
}
