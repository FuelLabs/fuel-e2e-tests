use crate::commands;
use crate::sway::project::SwayProject;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub struct CompilationError {
    pub project: SwayProject,
    pub reason: String,
}

impl Display for CompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Project '{:?}' failed to compile! Reason: {}",
            self.project.name(),
            self.reason
        )
    }
}

impl Error for CompilationError {}

impl SwayCompiler {
    pub fn new<T: Into<PathBuf>>(target_dir: T) -> SwayCompiler {
        SwayCompiler {
            target_dir: target_dir.into(),
        }
    }

    pub async fn build(&self, project: &SwayProject) -> Result<(), CompilationError> {
        let build_dir = self.prepare_project_dir(project).await?;

        commands::run_local_forc(project.path(), &build_dir)
            .await
            .map_err(|err| CompilationError {
                project: project.clone(),
                reason: err.to_string(),
            })?;

        Ok(())
    }

    async fn prepare_project_dir(
        &self,
        project: &SwayProject,
    ) -> Result<PathBuf, CompilationError> {
        let build_dir = self.target_dir.join(project.name());
        if build_dir.exists() {
            tokio::fs::remove_dir_all(&build_dir)
                .await
                .map_err(|_| CompilationError {
                    project: project.clone(),
                    reason: format!(
                        "Could not remove existing target dir for project '{build_dir:?}'"
                    ),
                })?;
        }

        Ok(build_dir)
    }
}

pub struct SwayCompiler {
    target_dir: PathBuf,
}
