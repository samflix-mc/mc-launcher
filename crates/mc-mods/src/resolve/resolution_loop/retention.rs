//! Keep a candidate, or give way to the one already there.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::resolve::choice::choose_build;
use crate::resolve::download::side_for;
use crate::resolve::plan::Installed;
use crate::resolve::queue::{Key, PendingRequest, ResolutionQueue};
use crate::resolve::reason::{authority, implicit_impasse};
use crate::resolve::{Options, Registry};

use super::arbitration::confront;
use super::derived::{complete_digests, push_dependencies};

/// Resolves a request and records it in the table of kept mods.
///
/// Three outcomes: the project is new and gets registered, it's already
/// there and holds its spot, or it's already there and gives way to a more
/// authoritative requester.
#[allow(clippy::too_many_arguments)]
pub(super) async fn retain(
    registry: &Registry,
    pending: PendingRequest,
    mc: &str,
    loader: &str,
    options: Options,
    chosen: &mut BTreeMap<Key, Installed>,
    queue: &mut ResolutionQueue,
    impasses: &mut BTreeSet<Key>,
) -> Result<()> {
    let (request, reason) = pending.into_parts();
    let mut candidate = choose_build(registry, &request, mc, loader).await?;
    complete_digests(&mut candidate, &request);

    tracing::debug!(
        slug = %candidate.slug,
        source = candidate.origin.as_str(),
        version = %candidate.version_number,
        file = %candidate.file_name,
        reason = %reason.describe(),
        "mod kept"
    );

    let id = (candidate.origin, candidate.project_id.clone());
    let incoming = authority(&reason, &request);
    let mut side = side_for(&request, &candidate);

    if let Some(existing) = chosen.get_mut(&id) {
        let same_build = same_build(existing, &candidate);
        if implicit_impasse(&reason, incoming, existing.authority, same_build) {
            impasses.insert(id.clone());
        }

        match confront(existing, &candidate, &reason, incoming, side, same_build) {
            None => return Ok(()),
            Some(merged) => {
                side = merged;
                // The dependencies declared by the discarded build no
                // longer have a requester: those of the winning build are
                // about to be pushed right after, and can be entirely
                // different.
                queue.forget_dependencies_of(&id);
            }
        }
    }

    let deps = if options.follow_declared {
        candidate.declared_deps.clone()
    } else {
        Vec::new()
    };
    let parent = candidate.name.clone();
    let source = candidate.origin;

    // Overwrites the entry if there is one: the path starts empty again, so
    // the right jar gets downloaded.
    chosen.insert(
        id.clone(),
        Installed {
            side,
            path: PathBuf::new(),
            provides: BTreeSet::new(),
            bundled: BTreeSet::new(),
            requires: Vec::new(),
            authority: incoming,
            reason,
            candidate,
        },
    );

    push_dependencies(queue, deps, source, &parent, &id);
    Ok(())
}

/// Do two requests name the same build?
///
/// It's the version identifier that says so, and only that: two candidates
/// can carry the same displayed number and come from different sources. The
/// answer decides two things — whether a replacement gets announced to the
/// player, and whether a requirement read from a jar has just lost its spot
/// for good. Getting it backwards would announce replacements that aren't
/// any, and record impasses where resolution had actually converged.
fn same_build(existing: &Installed, candidate: &crate::Candidate) -> bool {
    existing.candidate.version_id == candidate.version_id
}

#[cfg(test)]
#[path = "retention.test.rs"]
mod tests;
