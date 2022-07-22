use crate::run_local_forc;
use anyhow::bail;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::once;
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;
use tokio::io;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct SwayProject {
    name: String,
    path: PathBuf,
}

#[derive(Debug)]
pub struct CompilationError {
    pub project_name: String,
    pub reason: String,
}

impl Display for CompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Project '{:?}' failed to compile! Reason: {}",
            self.project_name, self.reason
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
        let build_dir = self.target_dir.join(project.name());
        run_local_forc(project.path(), &build_dir)
            .await
            .map_err(|err| CompilationError {
                project_name: project.name().to_string(),
                reason: err.to_string(),
            })?;

        Ok(())
    }
}

impl SwayProject {
    fn new(path: &Path) -> anyhow::Result<SwayProject> {
        if !path.join("Forc.toml").is_file() {
            bail!("{:?} does not contain a Forc.lock", path)
        }

        let path = path.canonicalize()?;
        let os_filename = path.file_name().expect(
            "Will not fail since we've canonicalized the path and thus it won't end in '..'",
        );
        let utf8_filename = os_filename
            .to_str()
            .expect("Don't see how a dir entry can have non utf-8 chars")
            .to_string();

        Ok(SwayProject {
            name: utf8_filename,
            path,
        })
    }

    pub async fn discover_projects(dir: &Path) -> anyhow::Result<Vec<SwayProject>> {
        let dir_entries = ReadDirStream::new(read_dir(dir).await?)
            .collect::<io::Result<Vec<_>>>()
            .await?;

        dir_entries
            .into_iter()
            .filter(|entry| Self::is_sway_project(&entry.path()))
            .map(|dir| SwayProject::new(&dir.path()))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    fn is_sway_project(dir: &Path) -> bool {
        dir.join("Forc.toml").is_file()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let source_entries = ReadDirStream::new(read_dir(self.path.join("src")).await?)
            .collect::<io::Result<Vec<_>>>()
            .await?;

        let files = source_entries
            .into_iter()
            .filter(|entry| matches!(entry.path().extension(), Some(ext) if ext == "sw"))
            .map(|entry| entry.path())
            .chain(once(self.path.join("Forc.toml")))
            .collect();

        Ok(files)
    }
}

pub struct SwayCompiler {
    target_dir: PathBuf,
}
