use crate::metadata::FsMetadata;
use crate::sway::project::{CompiledSwayProject, SwayProject};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredFingerprint {
    pub project_source: PathBuf,
    pub project_build: PathBuf,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprint {
    pub source: u32,
    pub build: u32,
}

impl Fingerprint {
    pub async fn of(compiled_project: &CompiledSwayProject) -> io::Result<Fingerprint> {
        let source_files = compiled_project.sway_project().source_files().await?;
        let source_fingerprint = fingerprint_files(source_files);

        let build_files = compiled_project.build_artifacts().await?;
        let build_fingerprint = fingerprint_files(build_files);

        Ok(Fingerprint {
            source: source_fingerprint,
            build: build_fingerprint,
        })
    }
}

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

fn fingerprint_files(mut vec: Vec<FsMetadata>) -> u32 {
    vec.sort_by(|left, right| left.path.cmp(&right.path));
    let filename_mtime_pairs = vec
        .into_iter()
        .map(|file| format!("{:?}:{:?}", file.path, file.modified))
        .join("\n");
    crc32fast::hash(filename_mtime_pairs.as_bytes())
}

pub async fn zip_with_fingerprints<T>(
    compiled_projects: T,
) -> io::Result<Vec<(CompiledSwayProject, Fingerprint)>>
where
    T: IntoIterator<Item = CompiledSwayProject>,
{
    tokio_stream::iter(compiled_projects.into_iter())
        .then(|project| async move {
            let fingerprint = Fingerprint::of(&project).await?;
            Ok((project, fingerprint))
        })
        .collect::<io::Result<Vec<_>>>()
        .await
}

#[cfg(test)]
mod tests {
    use crate::fingerprint::fingerprint_files;
    use crate::metadata::FsMetadata;
    use std::ops::Add;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn fingerprints_correctly() {
        let a_file = FsMetadata {
            path: "some/file.txt".into(),
            modified: UNIX_EPOCH,
        };
        let another_file = FsMetadata {
            path: "another/file.txt".into(),
            modified: UNIX_EPOCH.add(Duration::from_secs(10)),
        };

        let fingerprint = fingerprint_files(vec![a_file, another_file]);

        assert_eq!(fingerprint, 3304572981);
    }

    #[test]
    fn file_order_doesnt_matter() {
        let a_file = FsMetadata {
            path: "some/file.txt".into(),
            modified: UNIX_EPOCH,
        };
        let another_file = FsMetadata {
            path: "another/file.txt".into(),
            modified: UNIX_EPOCH.add(Duration::from_secs(10)),
        };

        let files = vec![a_file, another_file];
        let reversed_files = files.iter().cloned().rev().collect();

        let fingerprint_from_reverse = fingerprint_files(reversed_files);
        let normal_fingerprint = fingerprint_files(files);

        assert_eq!(normal_fingerprint, fingerprint_from_reverse);
    }
}

pub async fn fingerprint_and_save_to_file<'a, T: IntoIterator<Item = CompiledSwayProject>>(
    successful_projects: T,
    storage_file: &Path,
) -> anyhow::Result<()> {
    let to_store = prepare_for_storage(successful_projects).await?;

    let file = fs::File::create(storage_file)?;
    serde_json::to_writer_pretty(file, &to_store)?;

    Ok(())
}

async fn prepare_for_storage<T: IntoIterator<Item = CompiledSwayProject>>(
    successful_projects: T,
) -> io::Result<Vec<StoredFingerprint>> {
    let fingerprints = zip_with_fingerprints(successful_projects).await?;

    Ok(fingerprints
        .into_iter()
        .map(|(project, fingerprint)| StoredFingerprint {
            project_source: project.sway_project().path().to_path_buf(),
            project_build: project.build_path.to_path_buf(),
            fingerprint,
        })
        .collect())
}
