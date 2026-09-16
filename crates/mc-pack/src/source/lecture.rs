//! Reconnaître ce qu'on nous a donné, et aller le chercher.

use std::path::PathBuf;

use anyhow::Result;

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

use super::cache::{cache_dir_for, is_url};
use super::distant::load_remote;
use super::{Pack, Source};

impl Source {
    /// Une adresse se reconnaît à son protocole ; tout le reste est un chemin.
    ///
    /// Sans cette règle, installer depuis le dépôt et installer depuis le site
    /// demanderaient deux commandes, alors que c'est le même geste sur le même
    /// objet.
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

    /// Ce qu'on affiche pour dire d'où vient le pack.
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
