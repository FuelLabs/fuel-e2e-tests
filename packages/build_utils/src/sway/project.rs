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

impl SwayProject {
    pub fn new<T: AsRef<Path> + Debug + ?Sized>(path: &T) -> anyhow::Result<SwayProject> {
        let path = path.as_ref();

        if !path.join("Forc.toml").is_file() {
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

#[cfg(test)]
mod tests {
    use crate::sway::project::SwayProject;
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::fs::File;
    use std::hash::Hash;
    use std::iter;
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
            &workdir_path.join("main_project"),
            r#"
                [project]
                authors = ["Fuel Labs <contact@fuel.sh>"]
                entry = "main.sw"
                license = "Apache-2.0"
                name = "type_inside_enum"

                [dependencies]
                dep1 = { path = "../dep1" }
                dep2 = { path = "../dep2" }
        "#,
        )?;
        let project_dep1 = generate_sway_project(&workdir_path.join("dep1"), "")?;
        let project_dep2 = generate_sway_project(&workdir_path.join("dep2"), "")?;

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

        let real_project_1 = generate_sway_project(&workdir_path.join("real_project_1"), "")?;
        let real_project_2 = generate_sway_project(&workdir_path.join("real_project_2"), "")?;
        std::fs::create_dir(&workdir_path.join("some_folder"))?;

        // when
        let projects = SwayProject::discover_projects(workdir_path).await?;

        // then
        assert_contain_same_elements(&projects, &vec![real_project_1, real_project_2]);

        Ok(())
    }

    #[test]
    fn determine_project_name_from_non_canonical_path() -> anyhow::Result<()> {
        let workdir = tempdir()?;
        let workdir_path = workdir.path();
        let sut = generate_sway_project(&workdir_path.join("./a_dir/a_deeper_dir/.."), "")?;

        let project_name = sut.name();

        assert_eq!(project_name, "a_dir");

        Ok(())
    }

    #[tokio::test]
    async fn will_only_detect_sw_source_files() -> anyhow::Result<()> {
        let workdir = tempdir()?;
        let project_dir = workdir.path().join("some_sway_project");

        let some_sway_project = generate_sway_project(&project_dir, "")?;

        let valid_source_files =
            create_files(&project_dir, &["src/main.sw", "src/another_source.sw"])?;

        let not_source_files = create_files(&project_dir, &["src/some_random_file.txt"])?;

        let detected_source_files = some_sway_project
            .source_files()
            .await?
            .into_iter()
            .map(|metadata| metadata.path)
            .collect::<Vec<_>>();

        assert_contains(&detected_source_files, &valid_source_files);
        assert_doesnt_contain(&detected_source_files, &not_source_files);

        Ok(())
    }

    fn create_files(basedir: &Path, relative_paths: &[&str]) -> anyhow::Result<Vec<PathBuf>> {
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
        project_dir: &Path,
        forc_toml_contents: &str,
    ) -> anyhow::Result<SwayProject> {
        std::fs::create_dir_all(project_dir)?;
        std::fs::create_dir(project_dir.join("src"))?;

        std::fs::write(project_dir.join("Forc.toml"), forc_toml_contents)?;

        SwayProject::new(project_dir)
    }
}
