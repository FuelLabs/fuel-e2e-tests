use crate::sway::SwayProject;
use std::error::Error;
use utils::{
    compile_sway_projects, discover_all_files_related_to_projects, env_path, track_file_changes,
};

mod sway;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cargo_manifest = env_path("CARGO_MANIFEST_DIR")?;

    let projects_dir = cargo_manifest.join("tests/test_projects");
    let projects = SwayProject::discover_projects(&projects_dir).await?;

    for file in discover_all_files_related_to_projects(&projects).await? {
        track_file_changes(&file);
    }
    track_file_changes(&cargo_manifest.join("build/build.rs"));

    let out_dir = env_path("OUT_DIR")?;
    compile_sway_projects(projects, &out_dir.join("compiled_sway_projects")).await?;

    Ok(())
}
