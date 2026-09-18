//! Run the official installer, and make its failure readable.

use anyhow::{Context, Result, bail};
use std::path::Path;

pub(crate) async fn run_installer(
    installer: &Path,
    mode: &str,
    dir: &Path,
    java: &Path,
) -> Result<()> {
    let output = tokio::process::Command::new(java)
        .arg("-jar")
        .arg(installer)
        .arg(mode)
        .arg(dir)
        .current_dir(dir)
        .output()
        .await
        .with_context(|| format!("running {}", installer.display()))?;

    if !output.status.success() {
        // The installer writes its useful diagnostic to stdout and traces to
        // stderr; both are needed to understand a failure.
        bail!(
            "the NeoForge installer failed ({}):\n{}\n{}",
            output.status,
            tail(&String::from_utf8_lossy(&output.stdout), 25),
            tail(&String::from_utf8_lossy(&output.stderr), 25)
        );
    }
    Ok(())
}

/// Last lines of an output, where the gist of a failure is found.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
