use anyhow::anyhow;
use async_recursion::async_recursion;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io;
use tokio_stream::StreamExt;
use xtask::env_path;
use xtask::sway::{paths_in_dir, read_metadata, FileMetadata, SwayProject};
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

#[derive(Debug)]
struct ProjectFingerprint {
    current_fingerprint: ProjectFilesFingerprint,
    storage_fingerprint: Option<ProjectFilesFingerprint>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredFingerprint {
    project_path: PathBuf,
    fingerprint: ProjectFilesFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectFilesFingerprint {
    pub source: u32,
    pub build: u32,
}

struct Fingerprinter {
    target_dir: PathBuf,
    storage_fingerprints: HashMap<SwayProject, ProjectFilesFingerprint>,
}

fn load_stored_fingerprints<T: AsRef<Path>>(
    path: T,
) -> anyhow::Result<HashMap<SwayProject, ProjectFilesFingerprint>> {
    if !path.as_ref().exists() {
        return Ok(Default::default());
    }

    let file = fs::File::open(path)?;
    let stored_fingerprints: Vec<StoredFingerprint> = serde_json::from_reader(file)?;

    stored_fingerprints
        .into_iter()
        .map(
            |StoredFingerprint {
                 project_path,
                 fingerprint,
             }| SwayProject::new(&project_path).map(|project| (project, fingerprint)),
        )
        .collect::<Result<HashMap<SwayProject, ProjectFilesFingerprint>, _>>()
}

impl Fingerprinter {
    pub fn new(
        target_dir: PathBuf,
        storage_fingerprints: HashMap<SwayProject, ProjectFilesFingerprint>,
    ) -> Self {
        Self {
            target_dir,
            storage_fingerprints,
        }
    }

    pub async fn fingerprint(&self, project: &SwayProject) -> anyhow::Result<ProjectFingerprint> {
        let source_files = project.source_files().await?;
        let source_fingerprint = Self::fingerprint_files(source_files);

        let build_files = self.build_files(project).await?;
        let build_fingerprint = Self::fingerprint_files(build_files);

        let storage_fingerprint = self.storage_fingerprints.get(project).cloned();

        let current_fingerprint = ProjectFilesFingerprint {
            source: source_fingerprint,
            build: build_fingerprint,
        };

        Ok(ProjectFingerprint {
            current_fingerprint,
            storage_fingerprint,
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

struct DirtDetector {
    project_fingerprints: HashMap<SwayProject, ProjectFingerprint>,
}

impl DirtDetector {
    pub fn new(project_fingerprints: HashMap<SwayProject, ProjectFingerprint>) -> Self {
        Self {
            project_fingerprints,
        }
    }

    #[async_recursion]
    pub async fn is_dirty(&self, project: &SwayProject) -> anyhow::Result<bool> {
        let fingerprint = self.project_fingerprints.get(project).ok_or_else(|| {
            anyhow!("StaleDetector does not have fingerprint data on project {project:?}")
        })?;

        let fingerprints_changed = match &fingerprint.storage_fingerprint {
            None => true,
            Some(storage_fingerprint) => fingerprint.current_fingerprint != *storage_fingerprint,
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
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let xtask_dir = env_path!("CARGO_MANIFEST_DIR");
    let assets_dir = get_assets_dir(xtask_dir).await?;
    let projects = SwayProject::discover_projects(&xtask_dir.join("../tests/tests")).await?;

    let stored_fingerprints = load_stored_fingerprints("./storage.json").unwrap();
    let fingerprinter = Arc::new(Fingerprinter::new(assets_dir.clone(), stored_fingerprints));

    let fingerprints = fingerprint_projects(projects.clone(), &fingerprinter).await?;

    let detector = DirtDetector::new(fingerprints);

    let mut dirty_projects = vec![];
    for project in &projects {
        if detector.is_dirty(project).await? {
            dirty_projects.push(project.clone());
        }
    }

    let assets_dir_argument = &assets_dir;

    let project_list = dirty_projects
        .iter()
        .map(|project| format!("- {}", project.name()))
        .join("\n");
    eprintln!("Building Sway projects: \n{project_list}");

    let maybe_error = compile_sway_projects(&dirty_projects, assets_dir_argument)
        .await
        .err();

    let successful_projects = if let Some(compilation_errs) = maybe_error {
        let msg = compilation_errs
            .iter()
            .map(|err| format!("- {} - {}", err.project.name(), err.reason))
            .join("\n");

        eprintln!("Following Sway projects could not be built: \n{msg}");
        let failed_projects: Vec<_> = compilation_errs
            .into_iter()
            .map(|err| err.project)
            .collect();

        projects
            .into_iter()
            .filter(|project| !failed_projects.contains(project))
            .collect()
    } else {
        projects.clone()
    };

    let fingerprints = fingerprint_projects(successful_projects, &fingerprinter)
        .await?
        .into_iter()
        .map(|(a, b)| StoredFingerprint {
            project_path: a.path().to_path_buf(),
            fingerprint: b.current_fingerprint,
        })
        .collect::<Vec<_>>();

    let file = fs::File::create("./storage.json")?;
    serde_json::to_writer_pretty(file, &fingerprints)?;

    Ok(())
}

async fn fingerprint_projects(
    projects: Vec<SwayProject>,
    fingerprinter: &Fingerprinter,
) -> anyhow::Result<HashMap<SwayProject, ProjectFingerprint>> {
    let pairs = tokio_stream::iter(projects.into_iter())
        .then(|project| async move {
            let fingerprint = fingerprinter.fingerprint(&project).await?;
            Ok::<(SwayProject, ProjectFingerprint), anyhow::Error>((project, fingerprint))
        })
        .collect::<Result<Vec<_>, _>>()
        .await?;

    Ok(pairs.into_iter().collect())
}
