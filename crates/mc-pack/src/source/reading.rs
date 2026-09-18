//! Recognize what we were given, and go fetch it.

use std::path::PathBuf;

use anyhow::Result;

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

use super::cache::{cache_dir_for, is_url};
use super::remote::load_remote;
use super::{Pack, Source};

impl Source {
    /// An address is recognized by its protocol; everything else is a path.
    ///
    /// Without this rule, installing from the repo and installing from the
    /// site would need two commands, even though it's the same gesture on
    /// the same object.
    pub fn parse(arg: &str, layout: &mc_instance::Layout) -> Source {
        if is_url(arg) {
            Source::Remote {
                url: arg.to_string(),
                cache_dir: cache_dir_for(arg, layout),
            }
        } else {
            Source::File {
                manifest: PathBuf::from(arg),
            }
        }
    }

    /// What's displayed to say where the pack comes from.
    pub fn describe(&self) -> String {
        match self {
            Source::File { manifest } => manifest.display().to_string(),
            Source::Remote { url, .. } => url.clone(),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Source::Remote { .. })
    }

    pub async fn load(&self, dl: &mc_dl::Downloader) -> Result<Pack> {
        match self {
            Source::File { manifest } => {
                let lock_path = Lockfile::path_for(manifest);
                let lock = lock_path
                    .is_file()
                    .then(|| Lockfile::load(&lock_path))
                    .transpose()?;
                Ok(Pack {
                    manifest: Manifest::load(manifest)?,
                    lock,
                    lock_path,
                    replay: false,
                    from_cache: false,
                })
            }
            Source::Remote { url, cache_dir } => load_remote(url, cache_dir, dl).await,
        }
    }
}

#[cfg(test)]
#[path = "reading.test.rs"]
mod tests;
