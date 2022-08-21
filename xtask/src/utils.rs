use std::io;
use std::path::{Path, PathBuf};

use futures::{stream, StreamExt};
use itertools::Itertools;

use build_utils::commands::build_local_forc;
use build_utils::dirt_detector::DirtDetector;
use build_utils::env_path;
use build_utils::sway::compiler::{CompilationError, SwayCompiler};
use build_utils::sway::forc_runner::{BinaryForcRunner, CargoForcRunner, ForcRunner};
use build_utils::sway::project::{discover_projects, CompiledSwayProject, SwayProject};

pub async fn compile_sway_projects(
    projects: Vec<SwayProject>,
    target_dir: &Path,
    compile_forc: bool,
) -> anyhow::Result<(Vec<CompiledSwayProject>, Vec<CompilationError>)> {
    let runner = choose_forc_runner(compile_forc).await;

    let compiler = SwayCompiler::new(target_dir, runner);

    let result = stream::iter(projects)
        .map(|project| {
            let compiler = &compiler;
            async move {
                compiler
                    .build(&project)
                    .await
                    .map(|path| CompiledSwayProject::new(project, &path).unwrap())
            }
        })
        .buffer_unordered(num_cpus::get() * 2);

    Ok(result
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .partition_result())
}

async fn choose_forc_runner(compile_forc: bool) -> Box<dyn ForcRunner> {
    if compile_forc {
        build_forc_and_use_it().await
    } else {
        find_and_use_forc_binary_in_path()
    }
}

async fn build_forc_and_use_it() -> Box<dyn ForcRunner> {
    build_local_forc()
        .await
        .expect("Failed to build local forc! Investigate!");

    Box::new(CargoForcRunner)
}

fn find_and_use_forc_binary_in_path() -> Box<dyn ForcRunner> {
    let executables = find_forc_executables();

    let executable = match executables.as_slice() {
        [] => {
            panic!("Couldn't find a 'forc' binary in PATH.")
        }
        [only_executable] => only_executable,
        [first_executable, ..] => {
            eprintln!("Warning. Found multiple `forc` binaries in PATH: {executables:?}. Choosing: {first_executable:?}");
            first_executable
        }
    };

    Box::new(BinaryForcRunner::new(executable.clone()))
}

fn find_forc_executables() -> Vec<PathBuf> {
    which::which_all("forc").unwrap().collect()
}

pub async fn get_assets_dir() -> io::Result<PathBuf> {
    let assets_dir = env_path!("CARGO_MANIFEST_DIR").join("../assets");
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

pub fn adapt_error_message(compilation_errs: &[CompilationError]) -> String {
    let msg = compilation_errs
        .iter()
        .map(|err| format!("- {}, {}", err.project.name(), err.reason))
        .join("\n");

    format!("Following Sway projects could not be built: \n{msg}")
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
