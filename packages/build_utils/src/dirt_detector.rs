use futures::future::join_all;
use std::collections::HashMap;
use std::path::Path;

use crate::fingerprint::{load_stored_fingerprints, zip_with_fingerprints, Fingerprint};
use crate::sway::project::{CompiledSwayProject, SwayProject};

// Used to help determine which of the already compiled projects need
// recompiling nevertheless.
pub struct DirtDetector {
    storage_fingerprints: HashMap<CompiledSwayProject, Fingerprint>,
    current_fingerprints: HashMap<CompiledSwayProject, Fingerprint>,
    deps_info: HashMap<CompiledSwayProject, Vec<SwayProject>>,
}

impl DirtDetector {
    pub fn new<T, K, S>(storage_fingerprints: T, current_fingerprints: S, deps_info: K) -> Self
    where
        T: IntoIterator<Item = (CompiledSwayProject, Fingerprint)>,
        S: IntoIterator<Item = (CompiledSwayProject, Fingerprint)>,
        K: IntoIterator<Item = (CompiledSwayProject, Vec<SwayProject>)>,
    {
        Self {
            storage_fingerprints: storage_fingerprints.into_iter().collect(),
            current_fingerprints: current_fingerprints.into_iter().collect(),
            deps_info: deps_info.into_iter().collect(),
        }
    }

    /// Instantiates a `DirtDetector` by loading stored fingerprints from the
    /// file pointed to by `path` and proceeding to determine their current
    /// fingerprints and immediate dependencies.
    ///
    /// # Arguments
    ///
    /// * `file`: a file containing a JSON array of serialized
    /// `StoredFingerprint` object.
    pub async fn from_fingerprints_storage(file: &Path) -> anyhow::Result<DirtDetector> {
        let stored_fingerprints = load_stored_fingerprints(file)?;
        let compiled_projects = stored_fingerprints.keys().cloned();

        let current_fingerprints = zip_with_fingerprints(compiled_projects.clone());
        let deps_info = zip_with_deps(compiled_projects);

        let current_fingerprints = current_fingerprints.await?;
        let deps_info = deps_info.await?;

        Ok(Self::new(
            stored_fingerprints,
            current_fingerprints,
            deps_info,
        ))
    }

    /// From the `CompiledSwayProjects` originally read from storage, filter out
    /// those that don't need recompiling.
    pub fn get_clean_projects(&self) -> Vec<CompiledSwayProject> {
        self.storage_fingerprints
            .keys()
            .cloned()
            .into_iter()
            .filter(|project| !self.is_dirty(project))
            .collect()
    }

    fn get_compiled_project_from_storage(
        &self,
        project: &SwayProject,
    ) -> Option<&CompiledSwayProject> {
        self.storage_fingerprints
            .keys()
            .find(|compiled_project| compiled_project.sway_project() == project)
    }

    /// A project is dirty because its build or source files changed or because
    /// one of its dependencies became dirty.
    fn is_dirty(&self, compiled_project: &CompiledSwayProject) -> bool {
        if self.fingerprint_changed(compiled_project) {
            return true;
        }

        self.get_deps(compiled_project).iter().any(|project| {
            match self.get_compiled_project_from_storage(project) {
                None => true,
                Some(compiled_dep) => self.is_dirty(compiled_dep),
            }
        })
    }

    fn get_deps(&self, compiled_project: &CompiledSwayProject) -> &Vec<SwayProject> {
        self.deps_info
            .get(compiled_project)
            .unwrap_or_else(|| panic!("Missing deps info for project {compiled_project:?}"))
    }

    fn fingerprint_changed(&self, project: &CompiledSwayProject) -> bool {
        let current_fingerprint = self.current_fingerprints.get(project);

        if current_fingerprint.is_none() {
            panic!("Must have current fingerprint for {project:?} since we know that it exists.");
        }

        self.storage_fingerprints.get(project) != current_fingerprint
    }
}

/// Pairs each given project with a Vec of its dependencies.
async fn zip_with_deps<T>(
    compiled_projects: T,
) -> anyhow::Result<Vec<(CompiledSwayProject, Vec<SwayProject>)>>
where
    T: IntoIterator<Item = CompiledSwayProject>,
{
    let futures = compiled_projects
        .into_iter()
        .map(|project| async move {
            let deps = project.sway_project().deps().await?;
            Ok((project, deps))
        })
        .collect::<Vec<_>>();

    join_all(futures).await.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::dirt_detector::DirtDetector;
    use crate::fingerprint::Fingerprint;
    use crate::sway::project::{CompiledSwayProject, SwayProject};
    use crate::utils::test_utils::assert_contain_same_elements;
    use rand::{distributions::Alphanumeric, Rng};

    impl Fingerprint {
        pub fn slightly_changed(&self) -> Fingerprint {
            Fingerprint {
                source: self.source + 1,
                build: self.build + 1,
            }
        }
    }

    #[tokio::test]
    async fn project_is_dirty_if_its_fingerprint_changed() -> anyhow::Result<()> {
        // given
        let a_compiled_project = given_we_have_a_compiled_project();

        let stored_fingerprint = Fingerprint {
            source: 1,
            build: 1,
        };

        let storage_fingerprints = [(a_compiled_project.clone(), stored_fingerprint)];

        let current_fingerprints = [(
            a_compiled_project.clone(),
            stored_fingerprint.slightly_changed(),
        )];

        let deps_info = [(a_compiled_project, vec![])];

        let sut = DirtDetector::new(storage_fingerprints, current_fingerprints, deps_info);

        // when
        let clean_projects = sut.get_clean_projects();

        // then
        assert!(clean_projects.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn project_is_clean_if_its_fingerprint_is_unchanged() -> anyhow::Result<()> {
        // given
        let a_compiled_project = given_we_have_a_compiled_project();

        let storage_fingerprints: HashMap<_, _> = [(
            a_compiled_project.clone(),
            Fingerprint {
                source: 1,
                build: 1,
            },
        )]
        .into();

        let current_fingerprints = storage_fingerprints.clone();

        let deps_info = [(a_compiled_project.clone(), vec![])];

        let sut = DirtDetector::new(storage_fingerprints, current_fingerprints, deps_info);

        // when
        let clean_projects = sut.get_clean_projects();

        // then
        assert_eq!(clean_projects, vec![a_compiled_project]);

        Ok(())
    }

    #[tokio::test]
    async fn project_is_dirty_if_dep_changes_fingerprints() -> anyhow::Result<()> {
        // given
        let a_compiled_project = given_we_have_a_compiled_project();
        let a_compiled_dep = given_we_have_a_compiled_project();

        let original_fingerprint = Fingerprint {
            source: 0,
            build: 0,
        };
        let storage_fingerprints = [
            (a_compiled_project.clone(), Default::default()),
            (a_compiled_dep.clone(), original_fingerprint),
        ];

        let current_fingerprints = [
            (a_compiled_project.clone(), Default::default()),
            (
                a_compiled_dep.clone(),
                original_fingerprint.slightly_changed(),
            ),
        ];

        let deps_info = [(
            a_compiled_project,
            vec![a_compiled_dep.sway_project().clone()],
        )];

        let sut = DirtDetector::new(storage_fingerprints, current_fingerprints, deps_info);

        // when
        let clean_projects = sut.get_clean_projects();

        // then
        assert!(clean_projects.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn project_is_dirty_if_dep_not_in_fingerprint_storage() -> anyhow::Result<()> {
        // given
        let a_compiled_project = given_we_have_a_compiled_project();
        let a_compiled_dep = given_we_have_a_compiled_project();

        let storage_fingerprints = [(a_compiled_project.clone(), Default::default())];

        let current_fingerprints = [
            (a_compiled_project.clone(), Default::default()),
            (
                a_compiled_dep.clone(),
                Fingerprint {
                    source: 1,
                    build: 0,
                },
            ),
        ];

        let deps_info = [(
            a_compiled_project,
            vec![a_compiled_dep.sway_project().clone()],
        )];

        let sut = DirtDetector::new(storage_fingerprints, current_fingerprints, deps_info);

        // when
        let clean_projects = sut.get_clean_projects();

        // then
        assert!(clean_projects.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn project_is_clean_if_dep_is_unchanged() -> anyhow::Result<()> {
        // given
        let a_compiled_project = given_we_have_a_compiled_project();
        let a_compiled_dep = given_we_have_a_compiled_project();

        let storage_fingerprints = HashMap::from([
            (a_compiled_project.clone(), Default::default()),
            (a_compiled_dep.clone(), Default::default()),
        ]);

        let current_fingerprints = storage_fingerprints.clone();

        let deps_info = [
            (
                a_compiled_project.clone(),
                vec![a_compiled_dep.sway_project().clone()],
            ),
            (a_compiled_dep.clone(), vec![]),
        ];

        let sut = DirtDetector::new(storage_fingerprints, current_fingerprints, deps_info);

        // when
        let clean_projects = sut.get_clean_projects();

        // then
        assert_contain_same_elements(&clean_projects, &vec![a_compiled_project, a_compiled_dep]);

        Ok(())
    }

    fn given_we_have_a_compiled_project() -> CompiledSwayProject {
        let random_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        CompiledSwayProject::new_stub(
            SwayProject::new_stub(Path::new("some/project/").join(&random_name)),
            Path::new("some/built/project").join(&random_name),
        )
    }
}
