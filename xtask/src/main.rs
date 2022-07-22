use anyhow::anyhow;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use xtask::sway::SwayProject;
use xtask::utils::compile_sway_projects;
use xtask::{checked_command_wo_output_capture, env_path};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path!("CARGO_MANIFEST_DIR");

    let assets_dir = get_assets_dir(xtask_dir).await?;

    let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;

    compile_projects(&assets_dir, &projects).await?;

    run_tests().await?;

    Ok(())
}

async fn run_tests() -> anyhow::Result<()> {
    checked_command_wo_output_capture(env!("CARGO"), &["test", "--all", "--all-features"]).await
}

async fn compile_projects(assets_dir: &Path, projects: &[SwayProject]) -> anyhow::Result<()> {
    compile_sway_projects(projects, assets_dir)
        .await
        .map_err(|errors| {
            let msg = errors
                .iter()
                .map(|err| format!("- {} Reason: {}", err.project_name, err.reason))
                .join("\n");

            anyhow!("Errors while compiling sway projects: \n{msg}")
        })
}

async fn get_assets_dir(root_dir: &Path) -> std::io::Result<PathBuf> {
    let assets_dir = root_dir.join("../assets");
    tokio::fs::create_dir_all(&assets_dir).await?;
    Ok(assets_dir)
}
