use build_utils::commands::build_local_forc;
use build_utils::sway::compiler::{CompilationError, SwayCompiler};
use build_utils::sway::project::SwayProject;
use std::path::Path;
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

#[macro_export]
macro_rules! env_path {
    ($path:literal) => {{
        std::path::Path::new(env!($path))
    }};
}
