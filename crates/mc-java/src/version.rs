//! Ce qu'est un runtime, et comment on mesure sa version.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Un runtime Java repéré ou installé, dont la version a été **mesurée** en
/// exécutant le binaire — jamais déduite de son chemin.
#[derive(Debug, Clone)]
pub struct Java {
    /// Exécutable `java` (`java.exe` sous Windows).
    pub path: PathBuf,
    pub version: Version,
    pub origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Installé par le launcher dans son propre répertoire.
    Managed,
    /// Trouvé sur le système (`JAVA_HOME`, `PATH`, emplacements usuels).
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    /// Chaîne complète telle que rapportée, p. ex. `21.0.5+11`.
    pub full: String,
}

/// Extrait le numéro majeur d'une chaîne de version Java.
///
/// Deux schémas coexistent encore : `1.8.0_412` (jusqu'à Java 8, où le majeur
/// est le *deuxième* nombre) et `21.0.5` (depuis Java 9). Confondre les deux
/// ferait passer un Java 8 pour un Java 1.
pub fn parse_major(version: &str) -> Option<u32> {
    let cleaned: String = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .to_string();
    let mut parts = cleaned.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Interroge un exécutable `java` et lit sa version.
///
/// `-version` écrit sur **stderr** (choix historique de la JVM) et sur trois
/// lignes dont seule la première porte le numéro, entre guillemets.
pub async fn probe(exe: &Path) -> Result<Version> {
    let out = tokio::process::Command::new(exe)
        .arg("-version")
        .output()
        .await
        .with_context(|| format!("exécution de {}", exe.display()))?;

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let quoted = text
        .split('"')
        .nth(1)
        .with_context(|| format!("version illisible dans la sortie de {}", exe.display()))?;
    let major = parse_major(quoted)
        .with_context(|| format!("numéro majeur illisible dans « {quoted} »"))?;

    Ok(Version {
        major,
        full: quoted.to_string(),
    })
}

#[cfg(test)]
#[path = "version.test.rs"]
mod tests;
