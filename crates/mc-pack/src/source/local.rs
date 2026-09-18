//! The pack placed on this machine, not the one published.

use std::path::Path;

use anyhow::{Result, bail};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

use super::cache::file_name_of;
use super::{Pack, Source};

impl Source {
    /// The pack as it's placed on this machine, without touching the network.
    ///
    /// This is what `launch` and `verify` read, and for the same reason: both
    /// speak of the installation currently on disk. Going to fetch the
    /// published pack would make them describe a pack that isn't the one
    /// installed — and launching a session would stop working without a
    /// network, which is precisely the moment you want to play.
    ///
    /// It's `install` that refreshes the local copy, and only it.
    pub fn load_local(&self) -> Result<Pack> {
        let manifest_path = match self {
            Source::File { manifest } => manifest.clone(),
            Source::Remote { url, cache_dir } => {
                let cached = cache_dir.join(file_name_of(url));
                if !cached.is_file() {
                    bail!("pack {url} never installed — run `mc-pack install` first");
                }
                cached
            }
        };
        let lock_path = Lockfile::path_for(&manifest_path);
        let lock = lock_path
            .is_file()
            .then(|| Lockfile::load(&lock_path))
            .transpose()?;
        Ok(Pack {
            manifest: Manifest::load(&manifest_path)?,
            lock,
            lock_path,
            replay: self.is_remote(),
            from_cache: false,
        })
    }

    /// The manifest's path, when there is one to edit.
    ///
    /// `lock` needs it: resolving a remote pack would have nowhere to write
    /// its result.
    pub fn local_path(&self) -> Option<&Path> {
        match self {
            Source::File { manifest } => Some(manifest),
            Source::Remote { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "local.test.rs"]
mod tests;
