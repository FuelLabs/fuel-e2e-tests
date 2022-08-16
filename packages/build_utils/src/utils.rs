#[macro_export]
macro_rules! env_path {
    ($path:literal) => {{
        std::path::Path::new(env!($path))
    }};
}

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::sway::project::{CompiledSwayProject, SwayProject};
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::fs::File;
    use std::hash::Hash;
    use std::path::{Path, PathBuf};

    pub async fn extract_source_files(
        some_sway_project: &SwayProject,
    ) -> anyhow::Result<Vec<PathBuf>> {
        Ok(some_sway_project
            .source_files()
            .await?
            .into_iter()
            .map(|metadata| metadata.path)
            .collect())
    }

    /// For each given path, ensures that all of its components exist. I.e. if
    /// the directory is missing -- create it. Same goes for the file.
    pub fn ensure_files_exist(
        basedir: &Path,
        relative_paths: &[&str],
    ) -> anyhow::Result<Vec<PathBuf>> {
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

    pub fn assert_contain_same_elements<
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

    pub fn assert_contains<'a, T: IntoIterator<Item = &'a K>, K: Debug + PartialEq + 'a>(
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

    pub fn assert_doesnt_contain<'a, T: IntoIterator<Item = &'a K>, K: Debug + PartialEq + 'a>(
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

    pub fn generate_sway_project(
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

    pub fn generate_compiled_sway_project(
        sources_dir: &Path,
        project_name: &str,
        forc_toml_contents: &str,
        build_dir: &Path,
    ) -> anyhow::Result<CompiledSwayProject> {
        let project = generate_sway_project(sources_dir, project_name, forc_toml_contents)?;

        let dir = build_dir.join(project_name);
        std::fs::create_dir_all(&dir)?;

        CompiledSwayProject::new(project, &dir)
    }
}
