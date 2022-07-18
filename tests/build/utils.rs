use crate::sway::{SwayCompiler, SwayProject};
use anyhow::{anyhow, bail};
use futures::future::join_all;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::read_dir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

pub async fn compile_sway_projects(
    projects: Vec<SwayProject>,
    target_dir: &Path,
) -> anyhow::Result<()> {
    let shared_compiler = Arc::new(SwayCompiler::new(target_dir));

    let futures = projects
        .into_iter()
        .map(|project| {
            let compiler = Arc::clone(&shared_compiler);
            async move {
                let result = compiler.build(&project).await;
                eprintln!("Finished building {:?}", project.path());
                result
            }
        })
        .collect::<Vec<_>>();

    let results = join_all(futures).await;

    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        bail!("Errors while compiling: {:?}", errors)
    }

    Ok(())
}

pub fn env_path(env: &str) -> anyhow::Result<PathBuf> {
    Ok(std::env::var_os(env)
        .ok_or_else(|| anyhow!("Env variable '{}' not found!", env))?
        .into())
}

pub fn track_file_changes(file: &Path) {
    println!("cargo:rerun-if-changed={}", file.to_str().unwrap());
}

pub async fn discover_all_files_related_to_projects(
    projects: &[SwayProject],
) -> anyhow::Result<Vec<PathBuf>> {
    let files_to_track = projects
        .iter()
        .map(|project| async move { project.files().await })
        .collect::<Vec<_>>();

    let files_per_project = join_all(files_to_track).await;

    if files_per_project.iter().any(|files| files.is_err()) {
        let errors: Vec<_> = files_per_project
            .into_iter()
            .filter_map(|files| files.err())
            .collect();
        bail!(
            "Errors ocurred while scanning for project files: {:?}",
            errors
        )
    }

    Ok(files_per_project
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect::<Vec<_>>())
}

pub async fn all_rust_files_in(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let source_entries = ReadDirStream::new(read_dir(dir).await?)
        .collect::<io::Result<Vec<_>>>()
        .await?;

    let files = source_entries
        .into_iter()
        .filter_map(|entry| {
            let filepath = entry.path();
            if filepath.is_file() && matches!(filepath.extension(), Some(ext) if ext == "rs") {
                Some(filepath)
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}
