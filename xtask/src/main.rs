use xtask::checked_command_wo_output_capture;
use xtask::sway::SwayProject;
use xtask::utils::{compile_sway_projects, env_path};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path("CARGO_MANIFEST_DIR")?;

    let assets_dir = xtask_dir.join("../assets");
    tokio::fs::create_dir_all(&assets_dir).await?;

    let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;

    compile_sway_projects(projects, &assets_dir).await?;

    checked_command_wo_output_capture(env!("CARGO"), &["test", "--all", "--all-features"]).await?;

    Ok(())
}
