use crate::fingerprint::{Fingerprint, FingerprintCalculator};
use crate::sway::project::{CompiledSwayProject, SwayProject};
use async_recursion::async_recursion;
use std::collections::HashMap;
use tokio_stream::StreamExt;

pub struct DirtDetector {
    storage_fingerprints: HashMap<CompiledSwayProject, Fingerprint>,
}

impl DirtDetector {
    pub fn new(storage_fingerprints: HashMap<CompiledSwayProject, Fingerprint>) -> Self {
        Self {
            storage_fingerprints,
        }
    }

    fn to_compiled_project(&self, project: &SwayProject) -> Option<&CompiledSwayProject> {
        self.storage_fingerprints
            .keys()
            .find(|compiled_project| compiled_project.sway_project() == project)
    }

    #[async_recursion]
    async fn is_dirty(&self, compiled_project: &CompiledSwayProject) -> anyhow::Result<bool> {
        if self.fingerprint_changed(compiled_project).await? {
            return Ok(true);
        }

        for project in compiled_project.sway_project().deps().await? {
            match self.to_compiled_project(&project) {
                None => return Ok(true),
                Some(compiled_project) => {
                    if self.is_dirty(compiled_project).await? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    async fn fingerprint_changed(&self, project: &CompiledSwayProject) -> anyhow::Result<bool> {
        match self.storage_fingerprints.get(project) {
            Some(stored_fingerprint) => {
                let fingerprint = FingerprintCalculator::fingerprint(project).await?;

                Ok::<_, anyhow::Error>(fingerprint != *stored_fingerprint)
            }
            None => Ok(true),
        }
    }

    pub async fn get_clean_projects(&self) -> anyhow::Result<Vec<&CompiledSwayProject>> {
        Ok(tokio_stream::iter(self.storage_fingerprints.keys())
            .then(|project| async move {
                let clean_project = if !self.is_dirty(project).await? {
                    Some(project)
                } else {
                    None
                };
                Ok::<_, anyhow::Error>(clean_project)
            })
            .collect::<Result<Vec<_>, _>>()
            .await?
            .into_iter()
            .flatten()
            .collect())
    }
}
