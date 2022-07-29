use crate::run_local_forc;
use anyhow::bail;
use forc_pkg::ManifestFile;
use itertools::Itertools;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs::read_dir;
use tokio::io;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct SwayProject {
    path: PathBuf,
}

impl Display for SwayProject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone)]
pub struct CompiledSwayProject {
    project: SwayProject,
    target_dir: PathBuf,
}

impl CompiledSwayProject {
    pub fn new(project: SwayProject, target_dir: PathBuf) -> CompiledSwayProject {
        CompiledSwayProject {
            project,
            target_dir,
        }
    }

    pub fn project(&self) -> &SwayProject {
        &self.project
    }

    pub async fn source_files(&self) -> io::Result<Vec<FileMetadata>> {
        self.project.source_files().await
    }
    pub async fn build_files(&self) -> io::Result<Vec<FileMetadata>> {
        let build_dir_entries = ReadDirStream::new(read_dir(&self.target_dir).await?)
            .collect::<io::Result<Vec<_>>>()
            .await?;

        let build_files = build_dir_entries.into_iter().map(|entry| entry.path());

        read_metadata(build_files).await
    }
}

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

        run_local_forc(project.path(), &build_dir)
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

pub struct FileMetadata {
    pub path: PathBuf,
    pub modified: SystemTime,
}

pub async fn paths_in_dir(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let build_dir_entries = ReadDirStream::new(read_dir(dir).await?)
        .collect::<io::Result<Vec<_>>>()
        .await?;

    Ok(build_dir_entries
        .into_iter()
        .map(|entry| entry.path())
        .collect())
}

pub async fn read_metadata<T>(chain: T) -> Result<Vec<FileMetadata>, io::Error>
where
    T: IntoIterator<Item = PathBuf>,
{
    tokio_stream::iter(chain)
        .then(|path| async move {
            let modified = tokio::fs::metadata(&path).await?.modified()?;
            Ok::<FileMetadata, io::Error>(FileMetadata { path, modified })
        })
        .collect()
        .await
}

impl SwayProject {
    pub fn new<T: AsRef<Path> + Debug>(path: &T) -> anyhow::Result<SwayProject> {
        let path = path.as_ref();

        if !path.join("Forc.toml").is_file() {
            bail!("{:?} does not contain a Forc.lock", path)
        }

        let path = path.canonicalize()?;

        Ok(SwayProject { path })
    }

    pub async fn deps(&self) -> anyhow::Result<Vec<SwayProject>> {
        let manifest = ManifestFile::from_dir(&self.path, "UNUSED")?;

        Ok(manifest
            .deps()
            .filter_map(|(name, _)| manifest.dep_path(name))
            .map(|path| SwayProject::new(&path).unwrap())
            .collect())
    }

    pub async fn checksum(&self) -> anyhow::Result<u32> {
        let mut vec = self.source_files().await?;
        vec.sort_by(|left, right| left.path.cmp(&right.path));

        let to_track = vec
            .into_iter()
            .map(|file| format!("{:?}:{:?}", file.path, file.modified))
            .join("\n");

        Ok(crc32fast::hash(to_track.as_ref()))
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

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .expect(
                "Will not fail since we've canonicalized the path and thus it won't end in '..'",
            )
            .to_str()
            .expect("Don't see how a dir entry can have non utf-8 chars")
            .to_string()
    }

    pub async fn source_files(&self) -> io::Result<Vec<FileMetadata>> {
        let source_entries = ReadDirStream::new(read_dir(self.path.join("src")).await?)
            .collect::<io::Result<Vec<_>>>()
            .await?;

        let all_source_files = source_entries
            .into_iter()
            .map(|entry| entry.path())
            .filter(|path| matches!(path.extension(), Some(ext) if ext == "sw"))
            .chain(self.forc_files().into_iter());

        read_metadata(all_source_files).await
    }

    fn forc_files(&self) -> Vec<PathBuf> {
        ["Forc.lock", "Forc.toml"]
            .into_iter()
            .map(|filename| self.path.join(filename))
            .filter(|path| path.exists())
            .collect()
    }
}

pub struct SwayCompiler {
    target_dir: PathBuf,
}
