//! What arrives on disk, and how it gets there.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::progress::Progress;
use crate::{Check, Downloader, Fetched};

impl Downloader {
    /// Downloads `url` to `dest`, unless `dest` already satisfies `check`.
    ///
    /// What's written is **always** verified when a digest exists; `check`
    /// only sets the strictness of the check on a file already there.
    ///
    /// This is where the file name is known, so this is where it's
    /// announced: [`Downloader::bytes`] only sees a URL, and a CDN URL
    /// doesn't tell anyone anything.
    pub async fn to_file(&self, url: &str, dest: &Path, check: Check<'_>) -> Result<Fetched> {
        let name = short_name(dest);
        self.emit(Progress::Started {
            file: &name,
            bytes: check.size(),
        });

        if dest.is_file() && check.accepts_existing(dest) {
            tracing::trace!(file = %dest.display(), "already matching");
            // The size comes from disk rather than from `check`:
            // `Check::Full` doesn't publish one, and without it a file
            // already there wouldn't move the bar forward — a reinstall
            // would stay at zero from start to finish.
            //
            // `std::fs::metadata` and NOT `tokio::fs::metadata`, in a
            // function that's nonetheless `async`, and it's deliberate. This
            // path is the one for a file ALREADY matching: it runs several
            // thousand times on every check — one `stat` per asset object. A
            // `stat` costs a few microseconds; sending it to the
            // `spawn_blocking` pool would cost creating and scheduling a
            // task, which is more than the call itself, times three
            // thousand.
            //
            // The generic rule — "no blocking call in an async function" —
            // targets operations whose duration is unpredictable: reading or
            // writing content. That's why the WRITE, just below, goes
            // through `write_off_thread`.
            self.emit(Progress::Finished {
                file: &name,
                state: Fetched::AlreadyPresent,
                bytes: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0),
            });
            return Ok(Fetched::AlreadyPresent);
        }

        let bytes = self.bytes(url).await?;
        if let Some(sum) = check.checksum() {
            sum.verify(&bytes, &dest.display().to_string())?;
        } else if let Some(expected) = check.size() {
            // Absent a digest, size is the only check possible. It's enough
            // to rule out an error page or a redirect served over HTTP 200,
            // which is by far the most frequent case.
            if bytes.len() as u64 != expected {
                bail!(
                    "{}: {} bytes received, {expected} announced",
                    dest.display(),
                    bytes.len()
                );
            }
        }
        // The length BEFORE handing over the bytes: the detached task owns
        // them, and there's nothing left to measure afterward.
        let weight = bytes.len();
        write_off_thread(dest, bytes).await?;
        tracing::debug!(
            file = %dest.display(),
            bytes = weight,
            verified = check.checksum().is_some(),
            "downloaded"
        );
        self.emit(Progress::Finished {
            file: &name,
            state: Fetched::Downloaded,
            bytes: weight as u64,
        });
        Ok(Fetched::Downloaded)
    }
}

/// The file name alone, as it's displayed.
///
/// The full path would overflow the window and teach nothing: between
/// `~/.local/share/samflix-mc/shared/assets/objects/a3/a3f1…` and `a3f1…`,
/// only the latter fits on one line. Absent a name — a path that ends in
/// `..` — the full path beats nothing.
fn short_name(dest: &Path) -> String {
    dest.file_name()
        .unwrap_or(dest.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Writes to a `.part` file then renames: on the same file system, the
/// rename is atomic, so `dest` only exists once complete.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let part: PathBuf = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    std::fs::write(&part, bytes).with_context(|| format!("writing {}", part.display()))?;
    std::fs::rename(&part, dest).with_context(|| format!("renaming to {}", dest.display()))?;
    Ok(())
}

/// The same write, but **off the asynchronous execution thread**.
///
/// `write_atomic` blocks until the disk has responded. Called from an
/// `async` function, it blocks not the task but the tokio WORKER: while a
/// fifty-megabyte jar is being written, that thread advances no other task
/// — and downloads, which are launched concurrently, serialize behind the
/// slowest of them. On a first install, that adds up to minutes.
///
/// The `spawn_blocking` pool is separate from the one for tasks: that's
/// exactly what it exists for.
///
/// Takes the bytes by value rather than by reference: the detached task must
/// own what it writes, and the caller no longer needs them — it already
/// knew their length.
pub async fn write_off_thread(dest: &Path, bytes: Vec<u8>) -> Result<()> {
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || write_atomic(&dest, &bytes))
        .await
        .context("the detached write did not complete")?
}

/// Reads a file, off the asynchronous execution thread.
///
/// The counterpart of [`write_off_thread`], and for the same reason: a read
/// blocks the tokio worker, not just the task. It lives here rather than
/// with its callers so that `mc-news` — whose pure half has no async
/// dependency — doesn't have to pull in tokio for one line.
pub async fn read_off_thread(source: &Path) -> std::io::Result<Vec<u8>> {
    let source = source.to_path_buf();
    match tokio::task::spawn_blocking(move || std::fs::read(source)).await {
        Ok(result) => result,
        Err(error) => Err(std::io::Error::other(error)),
    }
}

#[cfg(test)]
#[path = "file.test.rs"]
mod tests;
