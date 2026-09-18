//! Finding who supplies a `modId` that no API announced.

mod suite;

use anyhow::Result;
use std::collections::BTreeSet;

use crate::Channel;
use crate::jar::Side;

use super::registry::filter::pick;
use super::request::Request;

use super::Registry;
use super::plan::Unresolved;
use super::queue::{Key, ResolutionQueue};

use suite::{CatchUp, catch_up_outcome};

/// Looks for a supplier for every `modId` that no API announced.
///
/// Output of [`resolve_with`]: this catch-up pass touches neither the table
/// of kept mods nor the pass count, it only feeds the queue — or the list
/// of gaps, when nobody can supply it.
pub(super) async fn catch_up(
    registry: &Registry,
    missing: Vec<(String, String, Side)>,
    mc: &str,
    loader: &str,
    impasses: &BTreeSet<Key>,
    queue: &mut ResolutionQueue,
    unresolved: &mut Vec<Unresolved>,
) -> Result<()> {
    for (mod_id, required_by, side) in missing {
        // The most useful trace of the lot: it names a dependency that
        // neither the manifest nor the API announced, and without which the
        // game wouldn't start.
        tracing::info!(
            mod_id = %mod_id,
            required_by = %required_by,
            "Implicit dependency: {required_by} requires \"{mod_id}\", which no API declared"
        );
        let found = registry.find_by_mod_id(&mod_id, mc, loader).await?;
        let request = Request {
            slug: mod_id.clone(),
            source: None,
            file: None,
            version: None,
            side: Some(side),
            channel: Some(Channel::Beta),
            expected_sha1: None,
            expected_sha512: None,
        };
        match catch_up_outcome(pick(found, &request), impasses, mod_id, required_by, side) {
            CatchUp::Enqueue(request, reason) => queue.push(request, reason, None),
            CatchUp::GiveUp(gap) => unresolved.push(gap),
        }
    }

    Ok(())
}
