use anyhow::anyhow;
use build_utils::commands::build_local_forc;
use build_utils::metadata::FileMetadata;
use build_utils::sway::{CompilationError, SwayCompiler, SwayProject};
use std::path::{Path, PathBuf};
use tokio_stream::StreamExt;

pub async fn compile_sway_projects(
    projects: &[&SwayProject],
    target_dir: &Path,
) -> anyhow::Result<Vec<CompilationError>> {
    build_local_forc()
        .await
        .expect("Failed to build local forc! Investigate!");

    let compiler = SwayCompiler::new(target_dir);

    let compilation_results = tokio_stream::iter(projects)
        .then(|project| {
            let compiler = &compiler;
            async move { compiler.build(project).await }
        })
        .collect::<Vec<_>>()
        .await;

    let errors = compilation_results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();

    Ok(errors)
}

pub fn env_path(env: &str) -> anyhow::Result<PathBuf> {
    Ok(std::env::var_os(env)
        .ok_or_else(|| anyhow!("Env variable '{}' not found!", env))?
        .into())
}

#[macro_export]
macro_rules! env_path {
    ($path:literal) => {{
        std::path::Path::new(env!($path))
    }};
}

pub async fn discover_all_files_related_to_projects(
    projects: &[SwayProject],
) -> anyhow::Result<Vec<FileMetadata>> {
    let files_per_project = tokio_stream::iter(projects)
        .then(|project| project.source_files())
        .collect::<Result<Vec<_>, _>>()
        .await?;

    Ok(files_per_project.into_iter().flatten().collect())
}
