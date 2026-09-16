//! Dépaqueter ce qu'Adoptium livre, quel qu'en soit le format.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub(crate) fn single_child(dir: &Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    match entries.len() {
        1 => Ok(entries.remove(0)),
        n => bail!(
            "archive Temurin inattendue : {n} entrées à la racine de {}",
            dir.display()
        ),
    }
}

pub(crate) fn extract(archive: &Path, into: &Path) -> Result<()> {
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    if name.ends_with(".zip") {
        extract_zip(archive, into)
    } else if name.ends_with(".tar.gz") {
        let file = std::fs::File::open(archive)?;
        tar::Archive::new(flate2::read::GzDecoder::new(file))
            .unpack(into)
            .with_context(|| format!("extraction de {name}"))?;
        Ok(())
    } else {
        bail!("format d'archive non géré : {name}")
    }
}

/// Extraction ZIP, pour la variante Windows d'Adoptium.
///
/// Les entrées sont validées avant écriture : une archive peut contenir des
/// chemins remontants (`../`) qui écriraient hors du répertoire cible.
fn extract_zip(archive: &Path, into: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let Some(relative) = entry.enclosed_name() else {
            bail!("entrée d'archive au chemin suspect : {}", entry.name());
        };
        let dest = into.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
}
