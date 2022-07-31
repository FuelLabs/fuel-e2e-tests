use crate::metadata::FileMetadata;
use crate::sway::project::SwayProject;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredFingerprint {
    pub project_path: PathBuf,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprint {
    pub source: u32,
    pub build: u32,
}

pub struct FingerprintCalculator {
    target_dir: PathBuf,
}

pub fn load_stored_fingerprints<T: AsRef<Path>>(
    path: T,
) -> anyhow::Result<HashMap<SwayProject, Fingerprint>> {
    if !path.as_ref().exists() {
        return Ok(Default::default());
    }

    Ok(
        serde_json::from_reader::<_, Vec<StoredFingerprint>>(fs::File::open(path)?)?
            .into_iter()
            .filter_map(
                |StoredFingerprint {
                     project_path,
                     fingerprint,
                 }| {
                    SwayProject::new(&project_path)
                        .map(|project| (project, fingerprint))
                        .ok()
                },
            )
            .collect::<HashMap<SwayProject, Fingerprint>>(),
    )
}

impl FingerprintCalculator {
    pub fn new(target_dir: PathBuf) -> Self {
        Self { target_dir }
    }

    pub async fn fingerprint(&self, project: &SwayProject) -> anyhow::Result<Fingerprint> {
        let source_files = project.source_files().await?;
        let source_fingerprint = Self::fingerprint_files(source_files);

        let build_files = self.build_files(project).await?;
        let build_fingerprint = Self::fingerprint_files(build_files);

        Ok(Fingerprint {
            source: source_fingerprint,
            build: build_fingerprint,
        })
    }

    fn fingerprint_files(mut vec: Vec<FileMetadata>) -> u32 {
        vec.sort_by(|left, right| left.path.cmp(&right.path));

        let filename_mtime_pairs = vec
            .into_iter()
            .map(|file| format!("{:?}:{:?}", file.path, file.modified))
            .join("\n");

        crc32fast::hash(filename_mtime_pairs.as_bytes())
    }

    async fn build_files(&self, project: &SwayProject) -> io::Result<Vec<FileMetadata>> {
        let project_build_dir = self.target_dir.join(project.name());

        if !project_build_dir.exists() {
            return Ok(vec![]);
        }

        let build_files = crate::metadata::paths_in_dir(&project_build_dir).await?;

        crate::metadata::read_metadata(build_files).await
    }
}
