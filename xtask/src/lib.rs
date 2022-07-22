use anyhow::bail;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;
use tokio::process::Command;

pub mod sway;
pub mod utils;

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

pub async fn checked_command_w_output_capture<T: AsRef<OsStr> + Debug>(
    command: &str,
    args: &[T],
) -> anyhow::Result<CommandOutput> {
    let output = Command::new(command)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await?;

    let stderr = to_colorless_string(&output.stderr)?;
    let status = output.status;

    if !status.success() {
        bail!("Running {command} {args:?} failed. Status: {status:?}. Message: {stderr}");
    }

    let stdout = to_colorless_string(&output.stderr)?;
    Ok(CommandOutput { stdout, stderr })
}

pub async fn checked_command_wo_output_capture<T: AsRef<OsStr> + Debug>(
    command: &str,
    args: &[T],
) -> anyhow::Result<()> {
    let status = Command::new(command)
        .args(args)
        .kill_on_drop(true)
        .status()
        .await?;

    if !status.success() {
        bail!("Running {command} {args:?} failed. Status: {status:?}");
    }

    Ok(())
}

fn to_colorless_string(bytes: &[u8]) -> anyhow::Result<String> {
    let bytes_w_no_color = strip_ansi_escapes::strip(&bytes)?;

    Ok(String::from_utf8_lossy(&bytes_w_no_color).into_owned())
}

pub async fn build_local_forc() -> anyhow::Result<CommandOutput> {
    checked_command_w_output_capture(
        env!("CARGO"),
        &["build", "--quiet", "--package", "local_forc"],
    )
    .await
}

pub async fn run_local_forc(
    project_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<CommandOutput> {
    checked_command_w_output_capture(
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
