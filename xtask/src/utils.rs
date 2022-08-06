use build_utils::commands::build_local_forc;
use build_utils::sway::compiler::{CompilationError, SwayCompiler};
use build_utils::sway::project::{CompiledSwayProject, SwayProject};
use itertools::Itertools;
use std::path::Path;
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
