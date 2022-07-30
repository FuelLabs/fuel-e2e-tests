use crate::fingerprint::{Fingerprint, FingerprintCalculator};
use crate::sway::SwayProject;
use async_recursion::async_recursion;
use std::collections::HashMap;
use tokio_stream::StreamExt;

pub struct DirtDetector<'a> {
    storage_fingerprints: HashMap<SwayProject, Fingerprint>,
    fingerprint_calculator: &'a FingerprintCalculator,
}

impl<'a> DirtDetector<'a> {
    pub fn new(
        storage_fingerprints: HashMap<SwayProject, Fingerprint>,
        fingerprint_calculator: &'a FingerprintCalculator,
    ) -> Self {
        Self {
            storage_fingerprints,
            fingerprint_calculator,
        }
    }

    #[async_recursion]
    async fn is_dirty(&self, project: &SwayProject) -> anyhow::Result<bool> {
        let fingerprints_changed = match self.storage_fingerprints.get(project).as_ref() {
            None => true,
            Some(storage_fingerprint) => {
                let fingerprint = self.fingerprint_calculator.fingerprint(project).await?;
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
