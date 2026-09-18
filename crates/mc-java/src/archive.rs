//! Unpack whatever Adoptium delivers, whatever the format.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub(crate) fn single_child(dir: &Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    match entries.len() {
        1 => Ok(entries.remove(0)),
        n => bail!(
            "unexpected Temurin archive: {n} entries at the root of {}",
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
            .with_context(|| format!("extracting {name}"))?;
        Ok(())
    } else {
        bail!("unsupported archive format: {name}")
    }
}

/// ZIP extraction, for Adoptium's Windows variant.
///
/// Entries are validated before writing: an archive can contain upward paths
/// (`../`) that would write outside the target directory.
fn extract_zip(archive: &Path, into: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let Some(relative) = entry.enclosed_name() else {
            bail!("archive entry with a suspicious path: {}", entry.name());
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

#[cfg(test)]
#[path = "archive.test.rs"]
mod tests;
