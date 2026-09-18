//! Where the session lives between two launches.
//!
//! The file contains a refresh token: whoever reads it can reopen the
//! player's session with no password and no second factor. It's therefore
//! written as `0600`, and never logged — `mc-log`'s redaction already
//! recognizes `refresh_token` and `access_token`, but it's still better not
//! to write it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Location of the session file.
///
/// Next to the CurseForge key, in the config directory and not the data
/// directory: it's a secret of the user's, not a rebuildable cache.
pub fn path() -> PathBuf {
    mc_paths::current().config.join("session.json")
}

/// The three operations take the path as an argument rather than reading it
/// from the environment: it's the only way to check that a corrupted file
/// doesn't prevent playing, and that the token is written `0600`, without
/// touching the real session of the machine running the tests.
///
/// An unreadable file isn't a fatal error: it means "no session", and the
/// caller will offer to sign in. The alternative would prevent playing
/// because of a corrupted file.
pub(crate) fn load_from(path: &Path) -> Option<serde_json::Value> {
    let raw = std::fs::read(path).ok()?;
    match serde_json::from_slice(&raw) {
        Ok(state) => Some(state),
        Err(error) => {
            tracing::warn!(
                file = %path.display(),
                error = %error,
                "saved session unreadable, sign-in required again"
            );
            None
        }
    }
}

pub(crate) fn save_to(path: &Path, state: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let raw = serde_json::to_vec_pretty(state).context("serializing the session")?;
    write_protected(path, &raw).with_context(|| format!("writing {}", path.display()))?;

    tracing::debug!(file = %path.display(), "session saved");
    Ok(())
}

pub(crate) fn erase_from(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("deleting {}", path.display())),
    }
}

/// Creates the file as `0600` **before** writing to it.
///
/// Writing then restricting would leave a window during which the token is
/// readable by everyone; on a shared machine, that window is enough.
#[cfg(unix)]
fn write_protected(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content)
}

/// Windows has no equivalent permission bit: the file inherits the
/// directory's rights, which are already under the user's profile.
///
/// Out of scope for mutation testing as long as CI runs on Linux: this body
/// is never compiled there, so no suite can exercise it. The day a Windows
/// runner is added, this attribute must go — it's the coverage that's
/// missing, not the mutant that's wrong.
#[cfg(not(unix))]
#[mutants::skip]
fn write_protected(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, content)
}

#[cfg(test)]
#[path = "file.test.rs"]
mod tests;
