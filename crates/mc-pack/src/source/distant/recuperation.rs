//! Aller chercher le pack et son verrou, et garder une copie.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::source::Pack;
use crate::source::cache::{file_name_of, lock_url_for};

pub(in crate::source) async fn load_remote(
    url: &str,
    cache_dir: &Path,
    dl: &mc_dl::Downloader,
) -> Result<Pack> {
    let lock_url = lock_url_for(url);
    let manifest_cache = cache_dir.join(file_name_of(url));
    let lock_cache = Lockfile::path_for(&manifest_cache);

    // Le manifeste et le verrou sont récupérés ensemble ou pas du tout : un
    // manifeste neuf accompagné du verrou d'avant décrirait un pack que
    // personne n'a jamais publié.
    let fetched = match fetch_pair(dl, url, &lock_url).await {
        Ok(pair) => Some(pair),
        Err(error) => {
            // « inutilisable » et non « injoignable » : le serveur peut très
            // bien avoir répondu, et servi une page d'erreur HTML là où on
            // attendait du JSON. Nommer la mauvaise cause fait chercher du
            // côté du réseau un défaut qui est côté contenu.
            tracing::warn!(
                url,
                erreur = %error,
                "Pack distant inutilisable, repli sur la dernière copie connue : {error}"
            );
            None
        }
    };

    if let Some((manifest, lock)) = fetched {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .with_context(|| format!("création du cache {}", cache_dir.display()))?;
        manifest.save(&manifest_cache)?;
        lock.save(&lock_cache)?;
        tracing::info!(
            pack = %manifest.name,
            mods = lock.mods.len(),
            "Pack « {} » repris depuis {url} — {} builds épinglés",
            manifest.name,
            lock.mods.len()
        );
        return Ok(Pack {
            manifest,
            lock: Some(lock),
            lock_path: lock_cache,
            replay: true,
            from_cache: false,
        });
    }

    if !copie_complete(&manifest_cache, &lock_cache) {
        bail!(
            "pack {url} inutilisable, et aucune copie dans {}",
            cache_dir.display()
        );
    }
    let manifest = Manifest::load(&manifest_cache)?;
    let lock = Lockfile::load(&lock_cache)?;
    tracing::warn!(
        pack = %manifest.name,
        genere = %lock.generated,
        "Hors-ligne : pack « {} » repris du cache, figé au {}",
        manifest.name,
        lock.generated
    );
    Ok(Pack {
        manifest,
        lock: Some(lock),
        lock_path: lock_cache,
        replay: true,
        from_cache: true,
    })
}

async fn fetch_pair(
    dl: &mc_dl::Downloader,
    manifest_url: &str,
    lock_url: &str,
) -> Result<(Manifest, Lockfile)> {
    let raw = dl.bytes(manifest_url).await?;
    let manifest = Manifest::parse(&raw)
        .with_context(|| format!("{manifest_url} ne contient pas un manifeste lisible"))?;
    let raw = dl.bytes(lock_url).await?;
    let lock = Lockfile::parse(&raw)
        .with_context(|| format!("{lock_url} ne contient pas un verrou lisible"))?;
    Ok((manifest, lock))
}

/// La copie locale est-elle exploitable ?
///
/// Il faut les deux fichiers : un manifeste sans son verrou ferait résoudre à
/// nouveau alors que le réseau est justement injoignable, et un verrou sans
/// son manifeste ne dit pas quel pack il verrouille. N'en exiger qu'un
/// laisserait l'installation continuer sur une moitié de pack.
fn copie_complete(manifest_cache: &std::path::Path, lock_cache: &std::path::Path) -> bool {
    manifest_cache.is_file() && lock_cache.is_file()
}

#[cfg(test)]
#[path = "recuperation.test.rs"]
mod tests;
