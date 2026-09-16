//! Retenir un runtime existant, ou en installer un.

use anyhow::Result;
use std::path::Path;

use crate::emplacements::{candidates, java_exe, managed_home};
use crate::installation::install;
use crate::version::{probe, Java, Origin};

pub async fn detect(major: u32, runtime_dir: &Path) -> Option<Java> {
    let managed = java_exe(&managed_home(runtime_dir, major));
    let mut seen = Vec::new();

    for exe in candidates(runtime_dir, major) {
        if !exe.is_file() {
            continue;
        }
        let real = tokio::fs::canonicalize(&exe)
            .await
            .unwrap_or_else(|_| exe.clone());
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);

        // Un exécutable peut être présent et cassé (paquet à moitié
        // désinstallé, lien symbolique mort) : on ne retient que ce qui répond.
        let Ok(version) = probe(&exe).await else {
            continue;
        };
        if version.major >= major {
            let origin = if exe == managed {
                Origin::Managed
            } else {
                Origin::System
            };
            return Some(Java {
                path: exe,
                version,
                origin,
            });
        }
    }
    None
}

/// Garantit la présence d'un Java ≥ `major` : détection, sinon installation.
#[tracing::instrument(name = "runtime java", skip(runtime_dir))]
pub async fn ensure(major: u32, runtime_dir: &Path) -> Result<Java> {
    if let Some(java) = detect(major, runtime_dir).await {
        tracing::debug!(
            version = %java.version.full,
            chemin = %java.path.display(),
            "runtime existant retenu"
        );
        return Ok(java);
    }
    // Le seul cas qui coûte du temps et de la bande passante : il mérite d'être
    // visible sans avoir à relever la verbosité.
    tracing::info!(
        majeur = major,
        "Aucun Java {major} sur ce poste, installation de Temurin"
    );
    install(major, runtime_dir).await
}
