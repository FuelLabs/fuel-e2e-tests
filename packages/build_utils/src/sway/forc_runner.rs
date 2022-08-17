use crate::commands::checked_command_drop_output;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait ForcRunner {
    async fn run_forc(&self, project_dir: &Path, output_dir: &Path) -> anyhow::Result<()>;
}

pub struct BinaryForcRunner {
    executable: PathBuf,
}

impl BinaryForcRunner {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }
}

#[async_trait]
impl ForcRunner for BinaryForcRunner {
    async fn run_forc(&self, project_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
        checked_command_drop_output(
            &self.executable,
            &[
                "build",
                "--silent",
                "--output-directory",
                &output_dir.to_string_lossy(),
                "--path",
                &project_dir.to_string_lossy(),
            ],
        )
        .await
    }
}

pub struct CargoForcRunner;

#[async_trait]
impl ForcRunner for CargoForcRunner {
    async fn run_forc(&self, project_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
        checked_command_drop_output(
            env!("CARGO"),
            &[
                "run",
                "--quiet",
                "--package",
                "local_forc",
                "--",
                "build",
                "--silent",
                "--output-directory",
                &output_dir.to_string_lossy(),
                "--path",
                &project_dir.to_string_lossy(),
            ],
        )
        .await
    }
}
