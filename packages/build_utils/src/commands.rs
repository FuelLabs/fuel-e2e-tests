use anyhow::bail;
use itertools::Itertools;
use std::ffi::OsStr;
use std::fmt::{Debug, Display};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

async fn checked_command_drop_output<T: AsRef<OsStr> + Debug + Display>(
    command: &str,
    args: &[T],
) -> anyhow::Result<()> {
    let status = Command::new(command)
        .args(args)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        let ws_separated_args = args.iter().join(" ");

        let command = format!("{} {}", command, ws_separated_args);
        bail!("Running command: '{command}' failed with status: {status}");
    }

    Ok(())
}

pub async fn checked_command_fwd_output<T: AsRef<OsStr> + Debug + Display>(
    command: &str,
    args: &[T],
    workdir: &Path,
) -> anyhow::Result<()> {
    let status = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .current_dir(workdir)
        .kill_on_drop(true)
        .status()
        .await?;

    if !status.success() {
        let ws_separated_args = args.iter().join(" ");

        let command = format!("{} {}", command, ws_separated_args);
        bail!("Running command: '{command}' failed with status: {status}");
    }

    Ok(())
}

pub async fn build_local_forc() -> anyhow::Result<()> {
    checked_command_drop_output(
        env!("CARGO"),
        &["build", "--quiet", "--package", "local_forc"],
    )
    .await
}

pub(crate) async fn run_local_forc(project_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
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
