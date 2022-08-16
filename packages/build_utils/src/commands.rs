use anyhow::bail;
use itertools::Itertools;
use std::ffi::OsStr;
use std::fmt::{Debug, Display};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Runs a process dropping its output and returning an Err in case of a
/// unsuccessful exit.
///
/// # Arguments
///
/// * `command`: The name of the executable you wish to run.
/// * `args`: Args to pass to the executable
pub(crate) async fn checked_command_drop_output<
    T: AsRef<OsStr> + Debug,
    K: AsRef<OsStr> + Debug,
>(
    command: T,
    args: &[K],
) -> anyhow::Result<()> {
    let status = Command::new(command.as_ref())
        .args(args)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        let ws_separated_args = args.iter().map(|arg| format!("{arg:?}")).join(" ");
        bail!("Running command: '{command:?} {ws_separated_args}' failed with status: {status}");
    }

    Ok(())
}

/// Runs a process forwarding its stdout and stderr to the current process's
/// stdout and stderr, respectfully. In case of a unsuccessful exit will return
/// an Err.
///
/// # Arguments
///
/// * `command`: The name of the executable you wish to run.
/// * `args`: Args to pass to the executable.
/// * `workdir`: The CWD of the newly spawned process.
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
