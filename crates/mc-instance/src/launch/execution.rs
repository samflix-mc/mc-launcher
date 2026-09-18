//! Launch the game, and report how it ended.

use anyhow::{Context, Result};

mod report;

use super::command::Command;

pub use report::{Outcome, Report};

/// Launches the game and waits for it to end.
pub async fn run(command: &Command) -> Result<Report> {
    run_observe(command, &|_| {}).await
}

/// The same, but reporting WHEN the game started and under which process id.
///
/// ## Why the process id comes out of here
///
/// Two needs, and only one way to serve them. The window has to know the
/// game is running — otherwise it keeps showing "Installing…" for the whole
/// session, which was observed during acceptance testing. And it has to be
/// able to stop it: a Minecraft that freezes on a loading screen never
/// returns control, and the launcher would then wait forever for a session
/// that will never end.
///
/// The `Child` stays HERE: it's borrowed by the loop that reads its output,
/// and sharing it would require a lock on the module's hottest path. A
/// process id is cheap to copy and is enough to signal.
#[tracing::instrument(name = "game execution", skip_all)]
pub async fn run_observe(
    command: &Command,
    on_launch: &(dyn Fn(u32) + Send + Sync),
) -> Result<Report> {
    tracing::info!(
        java = %command.java.display(),
        directory = %command.working_dir.display(),
        arguments = command.args.len(),
        "Starting Minecraft"
    );

    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // The output is captured to be read, then written back out as-is: the
    // player sees what they would have seen, and the launcher can pick out
    // exceptions along the way. The two streams are merged because
    // Minecraft writes to both without any meaningful distinction.
    let mut child = tokio::process::Command::new(&command.java)
        .args(&command.args)
        .current_dir(&command.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("running {}", command.java.display()))?;

    // Announced BEFORE reading a single line: the first one only arrives
    // once the JVM has started, which is several seconds later — exactly
    // the interval during which the window must already say the game is
    // leaving.
    match child.id() {
        Some(pid) => {
            tracing::info!(pid, "Minecraft started");
            on_launch(pid);
        }
        // A process with no id anymore has already ended. Nothing to
        // signal: the loop below will notice and render its verdict.
        None => tracing::warn!("the game has no process id"),
    }

    let mut watcher = crate::crash::Watcher::new();
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    let mut out_lines = stdout.map(|r| r.lines());
    let mut err_lines = stderr.map(|r| r.lines());

    loop {
        let line = tokio::select! {
            Ok(Some(line)) = async {
                match &mut out_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            Ok(Some(line)) = async {
                match &mut err_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            else => None,
        };
        match line {
            Some(line) => {
                println!("{line}");
                watcher.line(&line);
            }
            None => break,
        }
    }

    let status = child
        .wait()
        .await
        .with_context(|| format!("waiting for {}", command.java.display()))?;

    let outcome = Outcome::from_status(&status);
    // Nothing is logged as an error here: it's up to the caller to decide,
    // and it's the caller who knows the context. Doing it in both places
    // produced two separate incidents for a single failure, observed on
    // MC-LAUNCHER-4 and MC-LAUNCHER-5.
    tracing::debug!(?outcome, code = status.code(), "Minecraft exited");

    Ok(Report {
        outcome,
        errors: watcher.finish(),
    })
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
