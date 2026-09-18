//! Fetch the pack and its lock, and keep a copy.

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

    // The manifest and the lock are fetched together or not at all: a fresh
    // manifest paired with the previous lock would describe a pack nobody
    // has ever published.
    let fetched = match fetch_pair(dl, url, &lock_url).await {
        Ok(pair) => Some(pair),
        Err(error) => {
            // "unusable" and not "unreachable": the server may very well
            // have responded, and served an HTML error page where JSON was
            // expected. Naming the wrong cause sends the search toward the
            // network for a defect that's on the content side.
            tracing::warn!(
                url,
                error = %error,
                "Remote pack unusable, falling back to the last known copy: {error}"
            );
            None
        }
    };

    if let Some((manifest, lock)) = fetched {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .with_context(|| format!("creating cache {}", cache_dir.display()))?;
        manifest.save(&manifest_cache)?;
        lock.save(&lock_cache)?;
        tracing::info!(
            pack = %manifest.name,
            mods = lock.mods.len(),
            "Pack \"{}\" fetched from {url} — {} builds pinned",
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

    if !copy_complete(&manifest_cache, &lock_cache) {
        bail!(
            "pack {url} unusable, and no copy in {}",
            cache_dir.display()
        );
    }
    let manifest = Manifest::load(&manifest_cache)?;
    let lock = Lockfile::load(&lock_cache)?;
    tracing::warn!(
        pack = %manifest.name,
        generated = %lock.generated,
        "Offline: pack \"{}\" taken from the cache, frozen at {}",
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
        .with_context(|| format!("{manifest_url} doesn't contain a readable manifest"))?;
    let raw = dl.bytes(lock_url).await?;
    let lock = Lockfile::parse(&raw)
        .with_context(|| format!("{lock_url} doesn't contain a readable lock"))?;
    Ok((manifest, lock))
}

/// Is the local copy usable?
///
/// Both files are required: a manifest without its lock would resolve again
/// right when the network is exactly what's unreachable, and a lock without
/// its manifest doesn't say which pack it locks. Requiring only one would let
/// the installation carry on with half a pack.
fn copy_complete(manifest_cache: &std::path::Path, lock_cache: &std::path::Path) -> bool {
    manifest_cache.is_file() && lock_cache.is_file()
}

#[cfg(test)]
#[path = "fetch.test.rs"]
mod tests;
