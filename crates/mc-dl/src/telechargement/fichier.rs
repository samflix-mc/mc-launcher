//! Ce qui arrive sur le disque, et comment il y arrive.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::{Check, Downloader, Fetched};

impl Downloader {
    /// Télécharge `url` vers `dest`, sauf si `dest` satisfait déjà `check`.
    ///
    /// Ce qui est écrit est **toujours** vérifié quand une empreinte existe ;
    /// `check` ne règle que la sévérité du contrôle sur un fichier déjà là.
    pub async fn to_file(&self, url: &str, dest: &Path, check: Check<'_>) -> Result<Fetched> {
        if dest.is_file() && check.accepts_existing(dest) {
            tracing::trace!(fichier = %dest.display(), "déjà conforme");
            return Ok(Fetched::AlreadyPresent);
        }

        let bytes = self.bytes(url).await?;
        if let Some(sum) = check.checksum() {
            sum.verify(&bytes, &dest.display().to_string())?;
        } else if let Some(expected) = check.size() {
            // Faute d'empreinte, la taille est le seul contrôle possible. Il
            // suffit à écarter une page d'erreur ou une redirection servie en
            // HTTP 200, qui est le cas de loin le plus fréquent.
            if bytes.len() as u64 != expected {
                bail!(
                    "{} : {} octets reçus, {expected} annoncés",
                    dest.display(),
                    bytes.len()
                );
            }
        }
        write_atomic(dest, &bytes)?;
        tracing::debug!(
            fichier = %dest.display(),
            octets = bytes.len(),
            verifie = check.checksum().is_some(),
            "téléchargé"
        );
        Ok(Fetched::Downloaded)
    }
}

/// Écrit par `.part` puis renomme : sur le même système de fichiers, le
/// renommage est atomique, donc `dest` n'existe qu'une fois complet.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création de {}", parent.display()))?;
    }
    let part: PathBuf = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    std::fs::write(&part, bytes).with_context(|| format!("écriture de {}", part.display()))?;
    std::fs::rename(&part, dest).with_context(|| format!("renommage vers {}", dest.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "fichier.test.rs"]
mod tests;
