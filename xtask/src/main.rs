pub mod utils;

use build_utils::dirt_detector::DirtDetector;
use build_utils::fingerprint::{
    load_stored_fingerprints, FingerprintCalculator, StoredFingerprint,
};
use build_utils::sway::compiler::CompilationError;
use build_utils::sway::project::SwayProject;
use itertools::Itertools;
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
    let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;

    let fingerprinter = FingerprintCalculator::new(assets_dir.clone());

    let fingerprints_storage_path = Path::new("./storage.json");
    let stored_fingerprints = load_stored_fingerprints(fingerprints_storage_path).unwrap();
    let detector = DirtDetector::new(stored_fingerprints, &fingerprinter);

    let dirty_projects = detector.filter_dirty(&projects).await?;

    announce_build_started(&dirty_projects);

    let compilation_errors = compile_sway_projects(&dirty_projects, &assets_dir).await?;

    announce_build_finished(&compilation_errors);

    let successful_projects = filter_successful_projects(&projects, &compilation_errors);

    store_updated_fingerprints(
        &fingerprinter,
        &successful_projects,
        fingerprints_storage_path,
    )
    .await?;

    Ok(())
}

fn filter_successful_projects<'a>(
    projects: &'a [SwayProject],
    compilation_errs: &[CompilationError],
) -> Vec<&'a SwayProject> {
    let failed_projects: Vec<_> = compilation_errs.iter().map(|err| &err.project).collect();

    projects
        .iter()
        .filter(|project| !failed_projects.contains(project))
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

fn announce_build_started(project_to_build: &[&SwayProject]) {
    if !project_to_build.is_empty() {
        let project_list = project_to_build
            .iter()
            .map(|project| format!("- {}", project.name()))
            .join("\n");
        eprintln!("Building Sway projects: \n{project_list}");
    }
}

async fn fingerprint_projects_for_storage(
    projects: &[&SwayProject],
    fingerprinter: &FingerprintCalculator,
) -> anyhow::Result<Vec<StoredFingerprint>> {
    tokio_stream::iter(projects)
        .then(|project| async {
            let fingerprint = fingerprinter.fingerprint(project).await?;
            Ok(StoredFingerprint {
                project_path: project.path().to_path_buf(),
                fingerprint,
            })
        })
        .collect()
        .await
}

pub async fn store_updated_fingerprints<T: AsRef<Path>>(
    fingerprinter: &FingerprintCalculator,
    successful_projects: &[&SwayProject],
    storage_file: T,
) -> anyhow::Result<()> {
    let fingerprints_to_store =
        fingerprint_projects_for_storage(successful_projects, fingerprinter).await?;

    let file = fs::File::create(storage_file)?;
    serde_json::to_writer_pretty(file, &fingerprints_to_store)?;

    Ok(())
}
