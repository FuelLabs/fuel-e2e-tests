use async_recursion::async_recursion;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io;
use tokio_stream::StreamExt;
use xtask::env_path;
use xtask::sway::{paths_in_dir, read_metadata, CompilationError, FileMetadata, SwayProject};
use xtask::utils::compile_sway_projects;

// #[derive(Clone)]
// pub struct Fingerprint {
//     pub compiled_project: CompiledSwayProject,
//     pub source_fingerprint: u32,
//     pub build_fingerprint: u32,
// }
//
// impl Fingerprint {
//     pub async fn from(project: CompiledSwayProject) -> anyhow::Result<Fingerprint> {
//         let source_fingerprint = Self::fingerprint(project.source_files().await?);
//         let build_fingerprint = Self::fingerprint(project.build_files().await?);
//
//         Ok(Fingerprint {
//             compiled_project: project,
//             source_fingerprint,
//             build_fingerprint,
//         })
//     }
//
//     fn fingerprint(mut vec: Vec<FileMetadata>) -> u32 {
//         vec.sort_by(|left, right| left.path.cmp(&right.path));
//
//         let filename_mtime_pairs = vec
//             .into_iter()
//             .map(|file| format!("{:?}:{:?}", file.path, file.modified))
//             .join("\n");
//
//         crc32fast::hash(&filename_mtime_pairs.as_bytes())
//     }
// }

// #[tokio::main]
// async fn main() -> Result<(), anyhow::Error> {
//     // let xtask_dir = env_path!("CARGO_MANIFEST_DIR");
//     //
//     // let assets_dir = get_assets_dir(xtask_dir).await?;
//     //
//     // // svi projekti kojih nema u assets (sta je sa onih kojima je kompilacija neuspjesna?)
//     // // totalno novi projekti trebaju se kompajlirat
//     // // stari projekti sa novim fajlovima
//     // // stari projekti sa izbrisanim fajlovima
//     // // stari projekti sa modifikovanim fajlovima
//     // // ukratko ako se ista promjenilo na file listi projekta, rekompajl
//     // let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;
//     //
//     // let stored_checksums = fake_checksums();
//     //
//     // let current_checksums = current_checksums(&projects).await?;
//     //
//     // let new_projects: Vec<SwayProject> = filter_new_projects(&projects, &stored_checksums);
//     // let tampered_output_projects: Vec<SwayProject> =
//     //     filter_projects_w_tampered_output(&stored_checksums).await?;
//     //
//     // let modified_projects = filter_modified_projects(&stored_checksums);
//     //
//     // for project in projects {
//     //     println!("{project:?} - {}", project.checksum().await?);
//     // }
//     //
//     // // compile_projects(&assets_dir, &projects).await?;
//     // //
//     // // run_tests().await?;
//     //
//     // Ok(())
//     Ok(())
// }

//
async fn get_assets_dir(root_dir: &Path) -> io::Result<PathBuf> {
    let assets_dir = root_dir.join("../assets");
    tokio::fs::create_dir_all(&assets_dir).await?;
    Ok(assets_dir)
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredFingerprint {
    project_path: PathBuf,
    fingerprint: Fingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprint {
    pub source: u32,
    pub build: u32,
}

struct Fingerprinter {
    target_dir: PathBuf,
}

fn load_stored_fingerprints<T: AsRef<Path>>(
    path: T,
) -> anyhow::Result<HashMap<SwayProject, Fingerprint>> {
    if !path.as_ref().exists() {
        return Ok(Default::default());
    }

    serde_json::from_reader::<_, Vec<StoredFingerprint>>(fs::File::open(path)?)?
        .into_iter()
        .map(
            |StoredFingerprint {
                 project_path,
                 fingerprint,
             }| SwayProject::new(&project_path).map(|project| (project, fingerprint)),
        )
        .collect::<Result<HashMap<SwayProject, Fingerprint>, _>>()
}

impl Fingerprinter {
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

        let build_files = paths_in_dir(&project_build_dir).await?;

        read_metadata(build_files).await
    }
}

struct DirtDetector<'a> {
    storage_fingerprints: HashMap<SwayProject, Fingerprint>,
    fingerprinter: &'a Fingerprinter,
}

impl<'a> DirtDetector<'a> {
    pub fn new(
        storage_fingerprints: HashMap<SwayProject, Fingerprint>,
        fingerprinter: &'a Fingerprinter,
    ) -> Self {
        Self {
            storage_fingerprints,
            fingerprinter,
        }
    }

    #[async_recursion]
    async fn is_dirty(&self, project: &SwayProject) -> anyhow::Result<bool> {
        let fingerprints_changed = match self.storage_fingerprints.get(project).as_ref() {
            None => true,
            Some(storage_fingerprint) => {
                let fingerprint = self.fingerprinter.fingerprint(project).await?;
                fingerprint != **storage_fingerprint
            }
        };

        if fingerprints_changed {
            return Ok(true);
        }

        for project in project.deps().await? {
            if self.is_dirty(&project).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn filter_dirty(
        &self,
        projects: &'a [SwayProject],
    ) -> anyhow::Result<Vec<&'a SwayProject>> {
        tokio_stream::iter(projects)
            .then(|project| async move {
                let dirty_project = if self.is_dirty(project).await? {
                    Some(project)
                } else {
                    None
                };
                Ok::<_, anyhow::Error>(dirty_project)
            })
            .collect::<Result<Vec<_>, _>>()
            .await
            .map(|d| d.into_iter().flatten().collect())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path!("CARGO_MANIFEST_DIR");
    let assets_dir = get_assets_dir(xtask_dir).await?;
    let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;

    let stored_fingerprints = load_stored_fingerprints("./storage.json").unwrap();

    let fingerprinter = Fingerprinter::new(assets_dir.clone());
    let detector = DirtDetector::new(stored_fingerprints, &fingerprinter);

    let dirty_projects = detector.filter_dirty(&projects).await?;

    announce_building_of_projects(&dirty_projects);

    let compilation_errors = compile_sway_projects(&dirty_projects, &assets_dir)
        .await
        .err()
        .unwrap_or_default();

    announce_build_finished(&compilation_errors);

    let successful_projects = filter_successful_projects(&projects, &compilation_errors);

    store_updated_fingerprints(&fingerprinter, &successful_projects, "storage.json").await?;

    Ok(())
}

async fn store_updated_fingerprints<T: AsRef<Path>>(
    fingerprinter: &Fingerprinter,
    successful_projects: &[&SwayProject],
    storage_file: T,
) -> anyhow::Result<()> {
    let fingerprints_to_store =
        fingerprint_projects_for_storage(successful_projects, fingerprinter).await?;

    let file = fs::File::create(storage_file)?;
    serde_json::to_writer_pretty(file, &fingerprints_to_store)?;

    Ok(())
}

fn filter_successful_projects<'a>(
    projects: &'a [SwayProject],
    compilation_errs: &[CompilationError],
) -> Vec<&'a SwayProject> {
    let failed_projects: Vec<_> = compilation_errs.iter().map(|err| &err.project).collect();

    projects
        .iter()
        .filter(|project| !failed_projects.contains(project))
        .collect()
}

fn announce_build_finished(compilation_errs: &[CompilationError]) {
    if !compilation_errs.is_empty() {
        let msg = compilation_errs
            .iter()
            .map(|err| format!("- {} - {}", err.project.name(), err.reason))
            .join("\n");

        eprintln!("Following Sway projects could not be built: \n{msg}");
    }
}

fn announce_building_of_projects(dirty_projects: &[&SwayProject]) {
    let project_list = dirty_projects
        .iter()
        .map(|project| format!("- {}", project.name()))
        .join("\n");
    eprintln!("Building Sway projects: \n{project_list}");
}

async fn fingerprint_projects_for_storage(
    projects: &[&SwayProject],
    fingerprinter: &Fingerprinter,
) -> anyhow::Result<Vec<StoredFingerprint>> {
    tokio_stream::iter(projects)
        .then(|project| async {
            let fingerprint = fingerprinter.fingerprint(project).await?;
            Ok(StoredFingerprint {
                project_path: project.path().to_path_buf(),
                fingerprint,
            })
        })
        .collect()
        .await
}
