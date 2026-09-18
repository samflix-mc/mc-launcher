//! Download what a pass has kept, then read what the jars require.
//!
//! The two go together: a jar can only be read once it has come down, and
//! that reading is what decides whether another pass is needed.

use anyhow::Result;
use std::collections::BTreeMap;

use crate::resolve::Registry;
use crate::resolve::download::download_all;
use crate::resolve::inspection::inspect_all;
use crate::resolve::plan::Installed;
use crate::resolve::queue::Key;

pub(super) async fn download_and_read(
    registry: &Registry,
    chosen: &mut BTreeMap<Key, Installed>,
    pass: usize,
) -> Result<()> {
    let to_download = chosen
        .values()
        .filter(|m| m.path.as_os_str().is_empty())
        .count();
    let start = std::time::Instant::now();

    download_all(registry, chosen).await?;
    inspect_all(chosen)?;

    if worth_announcing(to_download) {
        tracing::info!(
            pass,
            jars = to_download,
            duration_ms = start.elapsed().as_millis(),
            "{to_download} jars downloaded and inspected in {} ms (pass {pass})",
            start.elapsed().as_millis()
        );
    }
    Ok(())
}

/// A pass that downloaded nothing has nothing to announce.
///
/// Resolution runs several passes, and the last ones often download no jar
/// at all: they only reread what the previous ones laid down. Announcing
/// "0 jars downloaded in 3 ms" on every one of them would drown out the
/// line that matters — and announcing nothing at all would deprive the
/// player of the only sign that the install is moving.
fn worth_announcing(to_download: usize) -> bool {
    to_download > 0
}

#[cfg(test)]
#[path = "descent.test.rs"]
mod tests;
