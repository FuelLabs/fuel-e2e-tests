pub mod utils;

use build_utils::dirt_detector::DirtDetector;
use build_utils::fingerprint::{
    load_stored_fingerprints, FingerprintCalculator, StoredFingerprint,
};
use build_utils::sway::compiler::CompilationError;
use build_utils::sway::project::{discover_projects, CompiledSwayProject, SwayProject};
use itertools::{chain, Itertools};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io;
use tokio_stream::StreamExt;
use utils::compile_sway_projects;

async fn get_assets_dir(root_dir: &Path) -> io::Result<PathBuf> {
    let assets_dir = root_dir.join("../assets");
    tokio::fs::create_dir_all(&assets_dir).await?;
    Ok(assets_dir)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path!("CARGO_MANIFEST_DIR");
    let assets_dir = get_assets_dir(xtask_dir).await?;
    let projects_dir = xtask_dir.join("../tests/tests");

    let storage_path = Path::new("./storage.json");

    let (clean_projects, dirty_projects) =
        detect_and_partition_projects(&projects_dir, storage_path).await?;

    announce_build_started(&dirty_projects);

    let (compiled, errors) = compile_sway_projects(&dirty_projects, &assets_dir).await?;

    announce_build_finished(&errors);

    store_updated_fingerprints(chain!(&compiled, &clean_projects), storage_path).await?;

    Ok(())
}

async fn detect_and_partition_projects(
    projects_dir: &Path,
    fingerprints_storage_path: &Path,
) -> anyhow::Result<(Vec<CompiledSwayProject>, Vec<SwayProject>)> {
    let stored_fingerprints = load_stored_fingerprints(fingerprints_storage_path).unwrap();

    let detector = DirtDetector::new(stored_fingerprints);

    let clean_projects = detector.get_clean_projects().await?;

    let projects = discover_projects(projects_dir).await?;
    let dirty_projects = filter_dirty_projects(projects, &clean_projects);

    Ok((
        clean_projects.into_iter().cloned().collect(),
        dirty_projects,
    ))
}

fn filter_dirty_projects<'a, T: IntoIterator<Item = &'a K>, K: AsRef<SwayProject> + 'a>(
    projects: Vec<SwayProject>,
    built_and_clean: T,
) -> Vec<SwayProject> {
    let built_and_clean: Vec<&SwayProject> =
        built_and_clean.into_iter().map(|p| p.as_ref()).collect();

    projects
        .into_iter()
        .filter(|p| !built_and_clean.contains(&p))
        .collect()
}

fn announce_build_finished(compilation_errs: &[CompilationError]) {
    if !compilation_errs.is_empty() {
        let msg = compilation_errs
            .iter()
            .map(|err| format!("- {} - {}", err.project.name(), err.reason))
            .join("\n");

        eprintln!("Following Sway projects could not be built: \n{msg}");
    }
}

fn announce_build_started(projects_to_build: &[SwayProject]) {
    if !projects_to_build.is_empty() {
        let project_list = projects_to_build
            .iter()
            .map(|project| format!("- {}", project.name()))
            .join("\n");
        eprintln!("Building Sway projects: \n{project_list}");
    }
}

pub async fn store_updated_fingerprints<'a, T: IntoIterator<Item = &'a CompiledSwayProject>>(
    successful_projects: T,
    storage_file: &Path,
) -> anyhow::Result<()> {
    let fingerprints_to_store = fingerprint_projects_for_storage(successful_projects).await?;

    let file = fs::File::create(storage_file)?;
    serde_json::to_writer_pretty(file, &fingerprints_to_store)?;

    Ok(())
}

async fn fingerprint_projects_for_storage<'a, T: IntoIterator<Item = &'a CompiledSwayProject>>(
    projects: T,
) -> anyhow::Result<Vec<StoredFingerprint>> {
    tokio_stream::iter(projects.into_iter())
        .then(|project| async {
            let fingerprint = FingerprintCalculator::fingerprint(project).await?;
            Ok(StoredFingerprint {
                project_source: project.project.path().to_path_buf(),
                project_build: project.target_path.to_path_buf(),
                fingerprint,
            })
        })
        .collect()
        .await
}
