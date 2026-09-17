//! Lancer l'installateur officiel, et rendre son échec lisible.

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
        .with_context(|| format!("exécution de {}", installer.display()))?;

    if !output.status.success() {
        // L'installateur écrit son diagnostic utile sur stdout et les traces
        // sur stderr ; les deux sont nécessaires pour comprendre un échec.
        bail!(
            "l'installateur NeoForge a échoué ({}) :\n{}\n{}",
            output.status,
            tail(&String::from_utf8_lossy(&output.stdout), 25),
            tail(&String::from_utf8_lossy(&output.stderr), 25)
        );
    }
    Ok(())
}

/// Dernières lignes d'une sortie, l'essentiel d'un échec s'y trouvant.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
