//! The libraries a version references, filtered by the rules.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::{Path, PathBuf};

use super::PARALLEL;
use super::descriptor::{Artifact, VersionJson};
use super::platform::{maven_path, mojang_arch, mojang_os};
use super::rules::allowed;

#[tracing::instrument(name = "libraries", skip_all)]
/// Out of scope for mutation testing: this function pulls down the libraries
/// the version descriptor lists, from Mojang's servers. Which ones apply to
/// this platform is verified by `rules`, and the download by mc-dl.
#[mutants::skip]
pub(super) async fn install_libraries(
    version: &VersionJson,
    shared: &Path,
    dl: &Downloader,
) -> Result<Vec<PathBuf>> {
    use futures_util::stream::{self, StreamExt};

    let os = mojang_os();
    let arch = mojang_arch();
    let root = shared.join("libraries");

    let wanted: Vec<(String, Artifact)> = version
        .libraries
        .iter()
        .filter(|lib| allowed(&lib.rules, os, arch))
        .filter_map(|lib| {
            let artifact = lib.downloads.as_ref()?.artifact.as_ref()?;
            let path = artifact.path.clone().or_else(|| maven_path(&lib.name))?;
            Some((
                path,
                Artifact {
                    path: None,
                    sha1: artifact.sha1.clone(),
                    size: artifact.size,
                    url: artifact.url.clone(),
                },
            ))
        })
        .collect();

    // The descriptor publishes the size of each library: the batch is
    // therefore known before the first one is requested. This is what makes
    // it possible to show a remaining time rather than an animation spinning
    // in the void.
    dl.emit(mc_dl::Progress::Batch {
        files: wanted.len(),
        bytes: weight(&wanted),
    });

    let results: Vec<Result<PathBuf>> = stream::iter(wanted)
        .map(|(path, artifact)| {
            let dest = root.join(&path);
            async move {
                dl.to_file(
                    &artifact.url,
                    &dest,
                    Check::Full(&Checksum::Sha1(artifact.sha1.clone())),
                )
                .await
                .with_context(|| format!("library {path}"))?;
                Ok(dest)
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    results.into_iter().collect()
}

/// What the batch weighs, before a single byte of it has been pulled down.
///
/// Separated from the loop to be verifiable: a wrong sum shows up nowhere
/// else than by watching a progress bar get it wrong.
fn weight(kept: &[(String, Artifact)]) -> u64 {
    kept.iter().map(|(_, artifact)| artifact.size).sum()
}

#[cfg(test)]
#[path = "libraries.test.rs"]
mod tests;
