use crate::metadata::FileMetadata;
use crate::sway::project::{CompiledSwayProject, SwayProject};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredFingerprint {
    pub project_source: PathBuf,
    pub project_build: PathBuf,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprint {
    pub source: u32,
    pub build: u32,
}

pub struct FingerprintCalculator;

pub fn load_stored_fingerprints<T: AsRef<Path>>(
    path: T,
) -> anyhow::Result<HashMap<CompiledSwayProject, Fingerprint>> {
    if !path.as_ref().exists() {
        return Ok(Default::default());
    }

    Ok(
        serde_json::from_reader::<_, Vec<StoredFingerprint>>(fs::File::open(path)?)?
            .into_iter()
            .filter_map(
                |StoredFingerprint {
                     project_source,
                     project_build,
                     fingerprint,
                 }| {
                    SwayProject::new(&project_source)
                        .and_then(|project| CompiledSwayProject::new(project, &project_build))
                        .map(|compiled_project| (compiled_project, fingerprint))
                        .ok()
                },
            )
            .collect(),
    )
}

impl FingerprintCalculator {
    pub async fn fingerprint(project: &CompiledSwayProject) -> anyhow::Result<Fingerprint> {
        let source_files = project.project.source_files().await?;
        let source_fingerprint = Self::fingerprint_files(source_files);

        let build_files = project.build_files().await?;
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
}
