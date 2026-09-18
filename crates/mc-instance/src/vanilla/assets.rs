//! The thousands of objects a version references.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::Path;

use super::descriptor::{AssetIndex, AssetIndexRef, AssetObject};
use super::{PARALLEL, RESOURCES};

#[tracing::instrument(name = "assets", skip_all, fields(index = %index.id))]
/// Out of scope for mutation testing: this function pulls down the thousands
/// of objects Mojang publishes, at an address written into this module. What
/// it counts is verifiable — see [`count_downloads`] — and the download
/// itself is verified in mc-dl.
#[mutants::skip]
pub(super) async fn install_assets(
    index: &AssetIndexRef,
    shared: &Path,
    dl: &Downloader,
) -> Result<usize> {
    use futures_util::stream::{self, StreamExt};

    let index_path = shared
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", index.id));
    dl.to_file(
        &index.url,
        &index_path,
        Check::Full(&Checksum::Sha1(index.sha1.clone())),
    )
    .await
    .context("downloading the asset index")?;

    let parsed: AssetIndex = serde_json::from_slice(&tokio::fs::read(&index_path).await?)
        .context("unreadable asset index")?;
    let objects = shared.join("assets").join("objects");

    // The index is read before starting: it's the only moment where we know
    // how much the longest step of the installation weighs. Without this
    // announcement, no remaining time is computable — bytes would arrive
    // without ever knowing how many are still missing.
    let expected: Vec<_> = parsed.objects.into_values().collect();
    dl.emit(mc_dl::Progress::Batch {
        files: expected.len(),
        bytes: weight(&expected),
    });

    let results: Vec<Result<mc_dl::Fetched>> = stream::iter(expected)
        .map(|object| {
            // Objects are addressed by their digest: two versions of the game
            // share everything that hasn't changed.
            let prefix = &object.hash[..2];
            let dest = objects.join(prefix).join(&object.hash);
            let url = format!("{RESOURCES}/{prefix}/{}", object.hash);
            async move {
                let sum = Checksum::Sha1(object.hash.clone());
                dl.to_file(
                    &url,
                    &dest,
                    Check::Quick {
                        sum: &sum,
                        size: object.size,
                    },
                )
                .await
                .with_context(|| format!("asset {}", object.hash))
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    let total = results.len();
    let downloaded = count_downloads(results)?;
    // The longest step of a first installation, and the quietest of a second
    // one: saying how many objects were skipped explains why.
    tracing::info!(
        total,
        downloaded,
        already_present = total - downloaded,
        "{downloaded} assets downloaded out of {total} ({} already present)",
        total - downloaded
    );
    Ok(downloaded)
}

/// What the batch weighs, before a single byte of it has been pulled down.
///
/// Separated from the loop to be verifiable: a wrong sum shows up nowhere
/// else than by watching a progress bar get it wrong.
fn weight(objects: &[AssetObject]) -> u64 {
    objects.iter().map(|object| object.size).sum()
}

/// How many objects were actually downloaded, out of the ones requested.
///
/// The first error encountered stops everything: a missing asset makes a
/// texture disappear, not a game that refuses to start, and we'd rather know
/// right away. The count, meanwhile, separates a first installation —
/// thousands of objects — from a second one, where everything is already
/// there: it's the only explanation we have for the wait.
fn count_downloads(results: Vec<Result<mc_dl::Fetched>>) -> Result<usize> {
    let mut downloaded = 0;
    for result in results {
        if result? == mc_dl::Fetched::Downloaded {
            downloaded += 1;
        }
    }
    Ok(downloaded)
}

#[cfg(test)]
#[path = "assets.test.rs"]
mod tests;
