use crate::utils::compile_sway_projects;
use build_utils::fingerprint;
use fingerprint::fingerprint_and_save_to_file;
use itertools::chain;
use std::path::Path;
use utils::{announce_build_finished, announce_build_started, detect_and_partition_projects};

pub mod utils;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path!("CARGO_MANIFEST_DIR");
    let assets_dir = utils::get_assets_dir(xtask_dir).await?;
    let projects_dir = xtask_dir.join("../tests/tests");

    let storage_path = Path::new("./storage.json");

    let (clean_projects, dirty_projects) =
        detect_and_partition_projects(&projects_dir, storage_path).await?;

    announce_build_started(&dirty_projects);

    let (compiled, errors) = compile_sway_projects(dirty_projects, &assets_dir).await?;

    announce_build_finished(&errors);

    fingerprint_and_save_to_file(chain!(compiled, clean_projects), storage_path).await?;

    Ok(())
}
