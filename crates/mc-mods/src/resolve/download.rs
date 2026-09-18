//! What comes down to disk, and at what pace.

use anyhow::{Context, Result};
use std::collections::BTreeMap;

use crate::jar::Side;
use crate::{Candidate, Origin};
use std::path::PathBuf;

use super::PARALLEL_DOWNLOADS;
use super::Registry;
use super::plan::Installed;
use super::request::Request;

/// Side kept: the manifest wins, otherwise the project's metadata.
pub(super) fn side_for(request: &Request, candidate: &Candidate) -> Side {
    request.side.unwrap_or(candidate.project_side)
}

/// A jar to download, pulled out of the plan before the loop starts.
///
/// Outside the function so that [`weight`] can get hold of it, and be
/// checked on its own.
struct Job {
    key: (Origin, String),
    url: String,
    file_name: String,
    sum: Option<mc_dl::Checksum>,
    size: u64,
}

/// What the batch weighs, before a single byte of it has come down.
fn weight(todo: &[Job]) -> u64 {
    todo.iter().map(|job| job.size).sum()
}

/// Downloads whatever has no path yet, in bounded parallel.
pub(super) async fn download_all(
    registry: &Registry,
    chosen: &mut BTreeMap<(Origin, String), Installed>,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};

    let todo: Vec<Job> = chosen
        .iter()
        .filter(|(_, m)| m.path.as_os_str().is_empty())
        .map(|(key, m)| Job {
            key: key.clone(),
            url: m.candidate.url.clone(),
            file_name: m.candidate.file_name.clone(),
            sum: m.candidate.checksum(),
            size: m.candidate.size,
        })
        .collect();

    // Sizes come from the project's metadata. CurseForge without a key
    // doesn't publish one and counts as zero then: the total is a floor,
    // not a promise — a bar that speeds up at the end beats no bar at all.
    registry.dl.emit(mc_dl::Progress::Batch {
        files: todo.len(),
        bytes: weight(&todo),
    });

    let results: Vec<Result<((Origin, String), PathBuf)>> = stream::iter(todo)
        .map(|job| {
            let dl = registry.dl.clone();
            // The cache is indexed by source and by project: two different
            // mods sometimes publish a jar under the same name.
            let dest = registry
                .cache
                .join(job.key.0.as_str())
                .join(&job.key.1)
                .join(&job.file_name);
            async move {
                // A jar is executed code: its digest is rechecked on every
                // pass, not just on write. Absent a digest, the announced
                // size is the only remaining safeguard.
                let check = match (&job.sum, job.size) {
                    (Some(sum), _) => mc_dl::Check::Full(sum),
                    (None, 0) => mc_dl::Check::Presence,
                    (None, size) => mc_dl::Check::Size(size),
                };
                dl.to_file(&job.url, &dest, check)
                    .await
                    .with_context(|| format!("downloading {}", job.file_name))?;
                Ok((job.key, dest))
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOADS)
        .collect()
        .await;

    for result in results {
        let (key, path) = result?;
        if let Some(entry) = chosen.get_mut(&key) {
            // Source without a digest: we compute our own. It goes into the
            // lock, and later installs get verified like all the others.
            if entry.candidate.checksum().is_none() {
                entry.candidate.sha512 = mc_dl::sha512_of_file(&path).ok();
            }
            entry.path = path;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "download.test.rs"]
mod tests;
