use crate::utils::{
    adapt_error_message, announce_build_started, compile_sway_projects,
    detect_and_partition_projects, get_assets_dir,
};
use anyhow::bail;
use build_utils::commands::checked_command_fwd_output;
use build_utils::env_path;
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
    Build {
        /// Instead of searching PATH for a 'forc' binary, compile it instead.
        #[clap(long, value_parser, default_value_t = false)]
        compile_forc: bool,
    },
    /// Builds (if necessary) sway projects and runs `cargo test` on the e2e tests.
    Test {
        /// Instead of searching PATH for a 'forc' binary, compile it instead.
        #[clap(long, value_parser, default_value_t = false)]
        compile_forc: bool,
        /// Run all workspace tests, not just e2e ones.
        #[clap(long, value_parser, default_value_t = false)]
        all: bool,
    },
}

/// Will determine what further action to take and take it.
pub async fn dispatch_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Build { compile_forc } => build(compile_forc).await,
        Commands::Test {
            compile_forc: use_forc_from_path,
            all,
        } => {
            build(use_forc_from_path).await?;
            test(all).await
        }
        Commands::Clean => clean().await,
    }
}

async fn build(compile_forc: bool) -> anyhow::Result<()> {
    let projects_dir = env_path!("CARGO_MANIFEST_DIR").join("../sway_projects");

    let storage_path = get_assets_dir().await?.join("storage.json");

    let (clean_projects, dirty_projects) =
        detect_and_partition_projects(&projects_dir, &storage_path).await?;

    announce_build_started(&dirty_projects);

    let (compiled, errors) =
        compile_sway_projects(dirty_projects, &get_assets_dir().await?, compile_forc).await?;

    fingerprint_and_save_to_file(chain!(compiled, clean_projects), &storage_path).await?;

    if !errors.is_empty() {
        bail!(adapt_error_message(&errors));
    }

    Ok(())
}

async fn test(all: bool) -> anyhow::Result<()> {
    let workspace_dir = env_path!("CARGO_MANIFEST_DIR").join("..");

    let additional_args = if all {
        vec!["--workspace"]
    } else {
        vec!["--package", "tests"]
    };

    let args = chain!(["test"], additional_args).collect::<Vec<_>>();

    checked_command_fwd_output(env!("CARGO"), &args, &workspace_dir).await
}

async fn clean() -> anyhow::Result<()> {
    let asset_dir = get_assets_dir().await?;
    fs::remove_dir_all(asset_dir).await?;
    Ok(())
}
