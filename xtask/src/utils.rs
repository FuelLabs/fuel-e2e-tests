use build_utils::commands::build_local_forc;
use build_utils::dirt_detector::DirtDetector;
use build_utils::sway::compiler::{CompilationError, SwayCompiler};
use build_utils::sway::project::{discover_projects, CompiledSwayProject, SwayProject};
use itertools::Itertools;
use std::io;
use std::path::{Path, PathBuf};
use tokio_stream::StreamExt;

pub async fn compile_sway_projects(
    projects: Vec<SwayProject>,
    target_dir: &Path,
) -> anyhow::Result<(Vec<CompiledSwayProject>, Vec<CompilationError>)> {
    build_local_forc()
        .await
        .expect("Failed to build local forc! Investigate!");

    let compiler = SwayCompiler::new(target_dir);

    let result = tokio_stream::iter(projects.into_iter())
        .then(|project| {
            let compiler = &compiler;
            async move {
                compiler
                    .build(&project)
                    .await
                    .map(|path| CompiledSwayProject::new(project, &path).unwrap())
            }
        })
        .collect::<Vec<_>>()
        .await;

    Ok(result.into_iter().partition_result())
}

#[macro_export]
macro_rules! env_path {
    ($path:literal) => {{
        std::path::Path::new(env!($path))
    }};
}

pub async fn get_assets_dir(root_dir: &Path) -> io::Result<PathBuf> {
    let assets_dir = root_dir.join("../assets");
    tokio::fs::create_dir_all(&assets_dir).await?;
    Ok(assets_dir)
}

pub async fn detect_and_partition_projects(
    projects_dir: &Path,
    fingerprints_storage_path: &Path,
) -> anyhow::Result<(Vec<CompiledSwayProject>, Vec<SwayProject>)> {
    let clean_projects = get_clean_projects(fingerprints_storage_path).await?;
    let dirty_projects = get_dirty_projects(projects_dir, &clean_projects).await?;

    Ok((clean_projects, dirty_projects))
}

async fn get_dirty_projects(
    projects_dir: &Path,
    clean_projects: &[CompiledSwayProject],
) -> anyhow::Result<Vec<SwayProject>> {
    let projects = discover_projects(projects_dir).await?;
    Ok(filter_dirty_projects(projects, clean_projects))
}

async fn get_clean_projects(
    fingerprints_storage_path: &Path,
) -> anyhow::Result<Vec<CompiledSwayProject>> {
    DirtDetector::from_fingerprints_storage(fingerprints_storage_path)
        .await
        .map(|detector| detector.get_clean_projects())
}

fn filter_dirty_projects(
    projects: Vec<SwayProject>,
    built_and_clean: &[CompiledSwayProject],
) -> Vec<SwayProject> {
    let built_and_clean: Vec<&SwayProject> =
        built_and_clean.iter().map(|p| p.sway_project()).collect();

    projects
        .into_iter()
        .filter(|p| !built_and_clean.contains(&p))
        .collect()
}

pub fn announce_build_finished(compilation_errs: &[CompilationError]) {
    if !compilation_errs.is_empty() {
        let msg = compilation_errs
            .iter()
            .map(|err| format!("- {} - {}", err.project.name(), err.reason))
            .join("\n");

        eprintln!("Following Sway projects could not be built: \n{msg}");
    }
}

pub fn announce_build_started(projects_to_build: &[SwayProject]) {
    if !projects_to_build.is_empty() {
        let project_list = projects_to_build
            .iter()
            .map(|project| format!("- {}", project.name()))
            .join("\n");
        eprintln!("Building Sway projects: \n{project_list}");
    }
}
