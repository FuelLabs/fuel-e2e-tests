use crate::metadata::FsMetadata;
use anyhow::bail;
use forc_pkg::ManifestFile;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;
use tokio::io;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwayProject {
    path: PathBuf,
}

impl SwayProject {
    pub fn new<T: AsRef<Path> + Debug + ?Sized>(path: &T) -> anyhow::Result<SwayProject> {
        let path = path.as_ref();

        if !contains_forc_toml(path) {
            bail!("{:?} does not contain a Forc.toml", path)
        }

        let path = path.canonicalize()?;

        Ok(SwayProject { path })
    }

    #[cfg(test)]
    pub fn new_stub<T: Into<PathBuf> + Debug>(path: T) -> SwayProject {
        SwayProject { path: path.into() }
    }

    pub async fn deps(&self) -> anyhow::Result<Vec<SwayProject>> {
        let manifest = ManifestFile::from_dir(&self.path, "UNUSED")?;

        manifest
            .deps()
            .filter_map(|(name, _)| manifest.dep_path(name))
            .map(|path| SwayProject::new(&path))
            .collect()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> String {
        let os_filename = self.path.file_name().expect(
            "Will not fail since we've canonicalized the path and thus it won't end in '..'",
        );

        let filename = os_filename
            .to_str()
            .unwrap_or_else(|| panic!("{os_filename:?} had non utf-8 chars"));

        filename.into()
    }

    pub async fn source_files(&self) -> io::Result<Vec<FsMetadata>> {
        let src_dir = self.path.join("src");

        let all_source_files = list_files_in(&src_dir)
            .await?
            .into_iter()
            .filter(|path| matches!(path.extension(), Some(ext) if ext == "sw"))
            .chain(self.forc_files().into_iter());

        FsMetadata::from_iter(all_source_files).await
    }

    fn forc_files(&self) -> Vec<PathBuf> {
        ["Forc.lock", "Forc.toml"]
            .into_iter()
            .map(|filename| self.path.join(filename))
            .filter(|path| path.exists())
            .collect()
    }
}

pub async fn discover_projects(dir: &Path) -> anyhow::Result<Vec<SwayProject>> {
    list_folders_in(dir)
        .await?
        .into_iter()
        .filter(|entry| contains_forc_toml(entry))
        .map(|dir| SwayProject::new(&dir))
        .collect::<anyhow::Result<Vec<_>>>()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompiledSwayProject {
    project: SwayProject,
    pub build_path: PathBuf,
}

impl CompiledSwayProject {
    pub fn new<T: Into<PathBuf>>(
        project: SwayProject,
        build_path: T,
    ) -> anyhow::Result<CompiledSwayProject> {
        let build_path = build_path.into();

        if !build_path.is_dir() {
            bail!("Failed to construct a CompiledSwayProject! {build_path:?} is not a directory!")
        }

        Ok(CompiledSwayProject {
            project,
            build_path,
        })
    }

    #[cfg(test)]
    pub fn new_stub<T: Into<PathBuf>>(project: SwayProject, build_path: T) -> CompiledSwayProject {
        CompiledSwayProject {
            project,
            build_path: build_path.into(),
        }
    }

    pub fn sway_project(&self) -> &SwayProject {
        &self.project
    }

    pub async fn build_artifacts(&self) -> io::Result<Vec<FsMetadata>> {
        let build_entries = list_entries_in(&self.build_path).await?;

        FsMetadata::from_iter(build_entries).await
    }

    pub fn build_path(&self) -> &Path {
        &self.build_path
    }
}

fn contains_forc_toml(dir: &Path) -> bool {
    dir.join("Forc.toml").is_file()
}

async fn list_folders_in(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    Ok(list_entries_in(dir)
        .await?
        .into_iter()
        .filter(|path| path.is_dir())
        .collect())
}

async fn list_files_in(dir: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(list_entries_in(dir)
        .await?
        .into_iter()
        .filter(|path| path.is_file())
        .collect())
}

async fn list_entries_in(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let dir_entries = ReadDirStream::new(read_dir(dir).await?)
        .collect::<io::Result<Vec<_>>>()
        .await?;

    Ok(dir_entries
        .into_iter()
        .map(|dir_entry| dir_entry.path())
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::metadata::FsMetadata;
    use crate::sway::project::{discover_projects, CompiledSwayProject, SwayProject};

    use crate::utils::test_utils::*;
    use std::fs::File;
    use tempfile::tempdir;

    mod testing_sway_project {
        use super::*;

        #[test]
        fn creating_a_sway_project_fails_if_forc_toml_not_present() -> anyhow::Result<()> {
            let sway_project_dir = tempdir()?;

            let err = SwayProject::new(&sway_project_dir)
                .expect_err("Should have failed because dir didn't contain Forc.toml");

            assert!(err.to_string().contains("does not contain a Forc.toml"));

            Ok(())
        }

        #[test]
        fn can_create_a_valid_sway_project() -> anyhow::Result<()> {
            let sway_project_dir = tempdir()?;
            File::create(sway_project_dir.path().join("Forc.toml"))?;

            SwayProject::new(&sway_project_dir)?;

            Ok(())
        }

        #[tokio::test]
        async fn can_read_deps_from_project() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let workdir_path = workdir.path();

            let sut = generate_sway_project(
                workdir_path,
                "main_project",
                r#"
                [project]
                authors = ["Fuel Labs <contact@fuel.sh>"]
                entry = "main.sw"
                license = "Apache-2.0"
                name = "main_project"

                [dependencies]
                dep1 = { path = "../dep1" }
                dep2 = { path = "../dep2" }
        "#,
            )?;
            let project_dep1 = generate_sway_project(workdir_path, "dep1", "")?;
            let project_dep2 = generate_sway_project(workdir_path, "dep2", "")?;

            // when
            let deps = sut.deps().await?;

            // then
            assert_contain_same_elements(&vec![project_dep1, project_dep2], &deps);

            Ok(())
        }

        #[tokio::test]
        async fn will_discover_sway_projects() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let workdir_path = workdir.path();

            let a_project = generate_sway_project(workdir_path, "a_project", "")?;

            std::fs::create_dir(&workdir_path.join("a_random_folder"))?;

            // when
            let projects = discover_projects(workdir_path).await?;

            // then
            assert_contain_same_elements(&projects, &vec![a_project]);

            Ok(())
        }

        #[test]
        fn determine_project_name_from_non_canonical_path() -> anyhow::Result<()> {
            let workdir = tempdir()?;
            let workdir_path = workdir.path();
            let sut = generate_sway_project(workdir_path, "./a_dir/a_deeper_dir/..", "")?;

            let project_name = sut.name();

            assert_eq!(project_name, "a_dir");

            Ok(())
        }

        #[tokio::test]
        async fn will_only_detect_sw_source_files() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let sut = generate_sway_project(workdir.path(), "project", "")?;
            let project_dir = sut.path();

            let valid_source_files =
                ensure_files_exist(project_dir, &["src/main.sw", "src/another_source.sw"])?;

            let not_source_files = ensure_files_exist(
                project_dir,
                &["src/some_random_file.txt", "some_root_file.txt"],
            )?;

            // when
            let detected_source_files = extract_source_files(&sut).await?;

            // then
            assert_contains(&detected_source_files, &valid_source_files);
            assert_doesnt_contain(&detected_source_files, &not_source_files);

            Ok(())
        }

        #[tokio::test]
        async fn will_detect_forc_files() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;

            let sut = generate_sway_project(workdir.path(), "project", "")?;
            let forc_files = ensure_files_exist(sut.path(), &["Forc.lock", "Forc.toml"])?;

            // when
            let detected_source_files = extract_source_files(&sut).await?;

            // then
            assert_contains(&detected_source_files, &forc_files);

            Ok(())
        }

        #[tokio::test]
        async fn source_files_will_contain_correct_metadata() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;

            let sut = generate_sway_project(workdir.path(), "", "")?;
            let expected_source_files =
                ensure_files_exist(sut.path(), &["src/main.sw", "Forc.toml"])?;

            let expected_mtimes = expected_source_files
                .iter()
                .cloned()
                .map(|path| {
                    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
                    (path, mtime)
                })
                .collect::<Vec<_>>();

            // when
            let detected_source_files = sut.source_files().await?;

            // then
            let actual_mtimes = detected_source_files
                .into_iter()
                .map(|FsMetadata { path, modified }| (path, modified))
                .collect::<Vec<_>>();

            assert_contain_same_elements(&expected_mtimes, &actual_mtimes);

            Ok(())
        }
    }

    mod testing_compiled_sway_project {
        use super::*;

        #[test]
        fn can_be_created() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let workdir_path = workdir.path();

            let sway_project =
                generate_sway_project(&workdir_path.join("source"), "a_project", "")?;

            let location_of_built_project = workdir_path.join("built_project");
            std::fs::create_dir(&location_of_built_project)?;

            // when
            let compiled_project =
                CompiledSwayProject::new(sway_project, &location_of_built_project);

            // then
            assert!(compiled_project.is_ok());

            Ok(())
        }

        #[test]
        fn build_path_must_point_to_dir() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let workdir_path = workdir.path();

            let sway_project =
                generate_sway_project(&workdir_path.join("source"), "a_project", "")?;

            // when
            let compiled_project = CompiledSwayProject::new(sway_project, "not_rly_a_dir");

            // then
            let err = compiled_project.expect_err("Should have failed since the dir doesn't exist");
            assert!(err.to_string().contains("is not a directory"));

            Ok(())
        }

        #[tokio::test]
        async fn will_list_build_artifacts() -> anyhow::Result<()> {
            // given
            let workdir = tempdir()?;
            let workdir_path = workdir.path();
            let sut = generate_compiled_sway_project(
                &workdir_path.join("source"),
                "a_project",
                "",
                &workdir_path.join("build"),
            )?;

            let project_build_dir = sut.build_path();

            let a_file_path = project_build_dir.join("a_file.txt");
            File::create(&a_file_path)?;

            let a_dir_path = project_build_dir.join("a_dir");
            std::fs::create_dir(&a_dir_path)?;

            // when
            let artifacts = sut.build_artifacts().await?;

            // then
            let artifact_paths = artifacts
                .into_iter()
                .map(|metadata| metadata.path)
                .collect();

            assert_contain_same_elements(&artifact_paths, &vec![a_file_path, a_dir_path]);

            Ok(())
        }
    }
}
