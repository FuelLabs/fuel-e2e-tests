use anyhow::{anyhow, bail};
use forc::test::{forc_build, BuildCommand};
use forc_pkg::Compiled;
use std::fs::File;
use std::iter::once;
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;
use tokio::io;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct SwayProject {
    name: String,
    path: PathBuf,
}

pub struct SwayCompiler {
    target_dir: PathBuf,
}

impl SwayCompiler {
    pub fn new<T: Into<PathBuf>>(target_dir: T) -> SwayCompiler {
        SwayCompiler {
            target_dir: target_dir.into(),
        }
    }

    pub async fn build(&self, project: &SwayProject) -> anyhow::Result<()> {
        {
            let compiled = Self::compile_project(project).await?;

            let build_dir = self.prepare_project_build_dir(project.name()).await?;

            self.write_binary(&compiled, &build_dir, project.name())
                .await?;

            self.write_abi(compiled, build_dir, project.name().to_string())
                .await
        }
        .map_err(|err| anyhow!("Error while building project {:?}: {}", project, err))
    }

    async fn write_abi(
        &self,
        compiled: Compiled,
        out_dir: PathBuf,
        project_name: String,
    ) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let filename = format!("{}-abi", project_name);
            let path = out_dir.join(&filename).with_extension("json");
            let file = File::create(path)?;

            serde_json::to_writer_pretty(&file, &compiled.json_abi)?;
            Ok(())
        })
        .await?
    }

    async fn write_binary(
        &self,
        compiled: &Compiled,
        out_dir: &Path,
        project_name: &str,
    ) -> io::Result<()> {
        tokio::fs::write(
            out_dir.join(project_name).with_extension("bin"),
            &compiled.bytecode,
        )
        .await
    }

    async fn prepare_project_build_dir(&self, project_name: &str) -> anyhow::Result<PathBuf> {
        let out_dir = self.target_dir.join(project_name);
        tokio::fs::create_dir_all(&out_dir).await?;
        Ok(out_dir)
    }

    async fn compile_project(project: &SwayProject) -> anyhow::Result<Compiled> {
        let path = project.path.to_str().unwrap().to_string();
        tokio::task::spawn_blocking(move || {
            forc_build::build(BuildCommand {
                path: Some(path),
                locked: false,
                silent_mode: false,
                ..Default::default()
            })
        })
        .await?
    }
}

impl SwayProject {
    fn new(path: &Path) -> anyhow::Result<SwayProject> {
        if !path.join("Forc.toml").is_file() {
            bail!("{:?} does not contain a Forc.lock", path)
        }

        let path = path.canonicalize()?;
        let os_filename = path.file_name().expect(
            "Will not fail since we've canonicalized the path and thus it won't end in '..'",
        );
        let utf8_filename = os_filename
            .to_str()
            .expect("Don't see how a dir entry can have non utf-8 chars")
            .to_string();

        Ok(SwayProject {
            name: utf8_filename,
            path,
        })
    }

    async fn discover_projects(dir: &Path) -> anyhow::Result<Vec<SwayProject>> {
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

    fn path(&self) -> &Path {
        &self.path
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let source_entries = ReadDirStream::new(read_dir(self.path.join("src")).await?)
            .collect::<io::Result<Vec<_>>>()
            .await?;

        let files = source_entries
            .into_iter()
            .filter(|entry| matches!(entry.path().extension(), Some(ext) if ext == "sw"))
            .map(|entry| entry.path())
            .chain(once(self.path.join("Forc.toml")))
            .collect();

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{SwayCompiler, SwayProject};
    use anyhow::bail;
    use futures::future::join_all;
    use std::path::Path;
    use std::sync::Arc;

    #[tokio::test]
    async fn something() -> anyhow::Result<()> {
        let dir =
            Path::new("/home/segfault_magnet/fuel/fuel_e2e/packages/fuel_e2e/tests/test_projects");

        let shared_compiler = Arc::new(SwayCompiler::new("output/sway_bins/"));
        let futures = SwayProject::discover_projects(dir)
            .await?
            .into_iter()
            .map(|project| {
                let compiler = Arc::clone(&shared_compiler);
                async move { compiler.build(&project).await }
            })
            .collect::<Vec<_>>();

        let results = join_all(futures).await;

        let errors = results
            .into_iter()
            .filter_map(|result| result.err())
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            bail!("Errors while compiling: {:?}", errors)
        }

        Ok(())
    }
}
