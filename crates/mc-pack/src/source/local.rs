//! Le pack posé sur cette machine, pas celui publié.

use std::path::Path;

use anyhow::{bail, Result};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

use super::cache::file_name_of;
use super::{Pack, Source};

impl Source {

    /// Le pack tel qu'il est posé sur cette machine, sans toucher au réseau.
    ///
    /// C'est ce que lisent `launch` et `verify`, et pour la même raison : tous
    /// deux parlent de l'installation présente sur le disque. Aller rechercher
    /// le pack publié les ferait décrire un pack qui n'est pas celui qu'on a
    /// installé — et lancer une partie cesserait de marcher sans réseau, ce
    /// qui est précisément le moment où l'on veut jouer.
    ///
    /// C'est `install` qui rafraîchit la copie locale, et lui seul.
    pub fn load_local(&self) -> Result<Pack> {
        let manifest_path = match self {
            Source::File { manifest } => manifest.clone(),
            Source::Remote { url, cache_dir } => {
                let cached = cache_dir.join(file_name_of(url));
                if !cached.is_file() {
                    bail!("pack {url} jamais installé — lancer « mc-pack install » d'abord");
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

    /// Le chemin du manifeste, quand il y en a un à éditer.
    ///
    /// `lock` en a besoin : résoudre un pack distant n'aurait nulle part où
    /// écrire son résultat.
    pub fn local_path(&self) -> Option<&Path> {
        match self {
            Source::File { manifest } => Some(manifest),
            Source::Remote { .. } => None,
        }
    }
}
