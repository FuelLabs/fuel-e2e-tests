use crate::metadata::{read_metadata, FileMetadata};
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
impl From<CompiledSwayProject> for SwayProject {
    fn from(compiled_project: CompiledSwayProject) -> Self {
        compiled_project.project
    }
}

async fn list_folders_in(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let dir_entries = ReadDirStream::new(read_dir(dir).await?)
        .collect::<io::Result<Vec<_>>>()
        .await?;

    Ok(dir_entries
        .into_iter()
        .map(|dir_entry| dir_entry.path())
        .filter(|path| path.is_dir())
        .collect())
}

pub async fn discover_projects(dir: &Path) -> anyhow::Result<Vec<SwayProject>> {
    list_folders_in(dir)
        .await?
        .into_iter()
        .filter(|entry| contains_forc_toml(entry))
        .map(|dir| SwayProject::new(&dir))
        .collect::<anyhow::Result<Vec<_>>>()
}

pub async fn discover_compiled_projects(
    projects: &[SwayProject],
    target_dir: &Path,
) -> anyhow::Result<Vec<CompiledSwayProject>> {
    projects
        .iter()
        .map(|project| (project, target_dir.join(project.name())))
        .filter(|(_, expected_build_dir)| expected_build_dir.is_dir())
        .map(|(project, build_dir)| CompiledSwayProject::new(project.clone(), &build_dir))
        .collect()
}

fn contains_forc_toml(dir: &Path) -> bool {
    dir.join("Forc.toml").is_file()
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompiledSwayProject {
    pub project: SwayProject,
    pub target_path: PathBuf,
}

impl AsRef<SwayProject> for CompiledSwayProject {
    fn as_ref(&self) -> &SwayProject {
        &self.project
    }
}

impl CompiledSwayProject {
    pub fn new(project: SwayProject, target_path: &Path) -> anyhow::Result<CompiledSwayProject> {
        let target_path = target_path.canonicalize()?;

        if !target_path.is_dir() {
            bail!("Failed to construct a CompiledSwayProject! {target_path:?} is not a directory!")
        }

        Ok(CompiledSwayProject {
            project,
            target_path,
        })
    }

    pub(crate) async fn build_files(&self) -> io::Result<Vec<FileMetadata>> {
        let build_files = crate::metadata::paths_in_dir(&self.target_path).await?;

        read_metadata(build_files).await
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::FileMetadata;
    use crate::sway::project::{
        discover_compiled_projects, discover_projects, CompiledSwayProject, SwayProject,
    };

    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::fs::File;
    use std::hash::Hash;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

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
    async fn will_discover_compiled_sway_projects() -> anyhow::Result<()> {
        // given
        let workdir = tempdir()?;
        let workdir_path = workdir.path();

        let src_dir = workdir_path.join("sources");
        std::fs::create_dir(&src_dir)?;

        let build_dir = workdir_path.join("build");
        std::fs::create_dir(&build_dir)?;

        let compiled_project =
            generate_compiled_sway_project(&src_dir, "a_compiled_project", "", &build_dir)?;

        let non_compiled_project = generate_sway_project(&src_dir, "a_non_compiled_project", "")?;
        std::fs::create_dir(&workdir_path.join("a_random_folder"))?;

        // when
        let projects = discover_compiled_projects(
            &[compiled_project.project.clone(), non_compiled_project],
            &build_dir,
        )
        .await?;

        // then
        assert_contain_same_elements(&projects, &vec![compiled_project]);

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
        let expected_source_files = ensure_files_exist(sut.path(), &["src/main.sw", "Forc.toml"])?;

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
            .map(|FileMetadata { path, modified }| (path, modified))
            .collect::<Vec<_>>();

        assert_contain_same_elements(&expected_mtimes, &actual_mtimes);

        Ok(())
    }

    async fn extract_source_files(some_sway_project: &SwayProject) -> anyhow::Result<Vec<PathBuf>> {
        Ok(some_sway_project
            .source_files()
            .await?
            .into_iter()
            .map(|metadata| metadata.path)
            .collect())
    }

    fn ensure_files_exist(basedir: &Path, relative_paths: &[&str]) -> anyhow::Result<Vec<PathBuf>> {
        relative_paths
            .iter()
            .map(|rel_path| {
                let path = basedir.join(rel_path);
                std::fs::create_dir_all(path.parent().unwrap())?;
                File::create(&path)?;
                Ok(path)
            })
            .collect()
    }

    fn assert_contain_same_elements<
        'a,
        T: IntoIterator<Item = &'a K>,
        K: Debug + PartialEq + Eq + Hash + 'a,
    >(
        left: T,
        right: T,
    ) {
        assert_eq!(
            left.into_iter().collect::<HashSet<_>>(),
            right.into_iter().collect::<HashSet<_>>()
        );
    }

    fn assert_contains<'a, T: IntoIterator<Item = &'a K>, K: Debug + PartialEq + 'a>(
        collection: T,
        elements_to_contain: T,
    ) {
        let collection = collection.into_iter().collect::<Vec<_>>();

        let missing_elements = elements_to_contain
            .into_iter()
            .filter(|el| !collection.contains(el))
            .collect::<Vec<_>>();

        assert!(
            missing_elements.is_empty(),
            "{collection:?} is missing these elements: {missing_elements:?}"
        );
    }

    fn assert_doesnt_contain<'a, T: IntoIterator<Item = &'a K>, K: Debug + PartialEq + 'a>(
        collection: T,
        elements_not_to_contain: T,
    ) {
        let collection = collection.into_iter().collect::<Vec<_>>();

        let offending_elements = elements_not_to_contain
            .into_iter()
            .filter(|el| collection.contains(el))
            .collect::<Vec<_>>();

        assert!(
            offending_elements.is_empty(),
            "{collection:?} should not contain these elements: {offending_elements:?}"
        );
    }

    fn generate_sway_project(
        parent_dir: &Path,
        project_name: &str,
        forc_toml_contents: &str,
    ) -> anyhow::Result<SwayProject> {
        let dir = parent_dir.join(project_name);

        std::fs::create_dir_all(&dir)?;
        std::fs::create_dir(dir.join("src"))?;

        std::fs::write(dir.join("Forc.toml"), forc_toml_contents)?;

        SwayProject::new(&dir)
    }

    fn generate_compiled_sway_project(
        sources_dir: &Path,
        project_name: &str,
        forc_toml_contents: &str,
        target_dir: &Path,
    ) -> anyhow::Result<CompiledSwayProject> {
        let project = generate_sway_project(sources_dir, project_name, forc_toml_contents)?;

        let dir = target_dir.join(project_name);
        std::fs::create_dir_all(&dir)?;

        CompiledSwayProject::new(project, &dir)
    }
}
