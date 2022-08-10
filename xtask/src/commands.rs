use crate::env_path;
use crate::utils::{
    adapt_error_message, announce_build_started, compile_sway_projects,
    detect_and_partition_projects, get_assets_dir,
};
use anyhow::bail;
use build_utils::commands::checked_command_fwd_output;
use build_utils::fingerprint::fingerprint_and_save_to_file;
use clap::{Parser, Subcommand};
use itertools::chain;
use tokio::fs;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deletes the asset dir along with the compiled sway projects.
    Clean,
    /// Builds sway projects and places the output in the 'assets' dir.
    Build,
    /// Builds (if necessary) sway projects and runs `cargo test` on the e2e tests.
    Test,
}

pub async fn dispatch_command(command: &Commands) -> anyhow::Result<()> {
    match command {
        Commands::Build => build().await,
        Commands::Test => {
            build().await?;
            test().await
        }
        Commands::Clean => clean().await,
    }
}

async fn build() -> anyhow::Result<()> {
    let projects_dir = env_path!("CARGO_MANIFEST_DIR").join("../tests/tests");

    let storage_path = get_assets_dir().await?.join("storage.json");

    let (clean_projects, dirty_projects) =
        detect_and_partition_projects(&projects_dir, &storage_path).await?;

    announce_build_started(&dirty_projects);

    let (compiled, errors) =
        compile_sway_projects(dirty_projects, &get_assets_dir().await?).await?;

    fingerprint_and_save_to_file(chain!(compiled, clean_projects), &storage_path).await?;

    if !errors.is_empty() {
        bail!(adapt_error_message(&errors));
    }

    Ok(())
}

async fn test() -> anyhow::Result<()> {
    let workspace_dir = env_path!("CARGO_MANIFEST_DIR").join("..");
    checked_command_fwd_output(env!("CARGO"), &["test", "--workspace"], &workspace_dir).await
}

async fn clean() -> anyhow::Result<()> {
    let asset_dir = get_assets_dir().await?;
    fs::remove_dir_all(asset_dir).await?;
    Ok(())
}
