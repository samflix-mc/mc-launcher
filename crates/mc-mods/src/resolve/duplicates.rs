//! Two projects, one same **root** `modId`: NeoForge will only load one.
//!
//! "Root" isn't a detail, it's the fix for a bug that cost the samflix pack
//! three mods. A jar declares its `modId`(s) in its descriptor, and brings
//! others through the libraries it bundles. Only the former say who it is;
//! the latter, NeoForge knows how to deduplicate on its own at load time.
//!
//! Confusing the two made Sodium and Iris look like a duplicate — they
//! share `fabric_api_base` and three other shims — and removed one of the
//! two, silently, with a zero exit code.

use std::collections::BTreeMap;

use crate::Origin;
use crate::resolve::reason::Reason;

use super::plan::Installed;

/// What a deduplication pass removed, and why.
///
/// Returned rather than logged: a mod that disappears from the plan must be
/// visible. The resolver used to return 0 having lost half a manifest, and
/// only mc-content's CI noticed, after the fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Eviction {
    /// Slug of the discarded project.
    pub discarded: String,
    /// Slug of the project that keeps the spot.
    pub kept: String,
    /// The root `modId` both declare.
    pub mod_id: String,
    /// Was the discarded project requested by the manifest?
    pub explicit: bool,
}

/// Discards projects that declare the same **root** `modId`.
///
/// Only the descriptor's `modId`s count, never the bundled jars' own: two
/// mods legitimately bundle the same library, and NeoForge knows how to
/// deduplicate them at load time. Confusing the two made Sodium and Iris
/// look like a duplicate — they share four Fabric shims — and silently
/// removed one of the two, even though a pack that keeps Iris without
/// Sodium is broken.
///
/// Between two claimants, keep the more verifiable one, then the more
/// intentional one: the one carrying a digest, then the one requested
/// explicitly.
pub(super) fn deduplicate_by_mod_id(
    chosen: &mut BTreeMap<(Origin, String), Installed>,
) -> Vec<Eviction> {
    let mut owner: BTreeMap<String, (Origin, String)> = BTreeMap::new();
    let mut drop_keys: Vec<(Origin, String)> = Vec::new();
    let mut evictions: Vec<Eviction> = Vec::new();

    for (key, entry) in chosen.iter() {
        for mod_id in &entry.provides {
            match owner.get(mod_id) {
                None => {
                    owner.insert(mod_id.clone(), key.clone());
                }
                Some(previous) => {
                    let keep_previous = {
                        let other = &chosen[previous];
                        let score = |m: &Installed| {
                            (
                                m.candidate.checksum().is_some(),
                                m.reason == Reason::Explicit,
                            )
                        };
                        score(other) >= score(entry)
                    };
                    let (loser, winner) = if keep_previous {
                        (key.clone(), previous.clone())
                    } else {
                        let loser = previous.clone();
                        owner.insert(mod_id.clone(), key.clone());
                        (loser, key.clone())
                    };
                    if !drop_keys.contains(&loser) {
                        let discarded = &chosen[&loser];
                        evictions.push(Eviction {
                            discarded: discarded.candidate.slug.clone(),
                            kept: chosen[&winner].candidate.slug.clone(),
                            mod_id: mod_id.clone(),
                            explicit: discarded.reason == Reason::Explicit,
                        });
                        drop_keys.push(loser);
                    }
                }
            }
        }
    }

    for key in drop_keys {
        chosen.remove(&key);
    }
    evictions
}

#[cfg(test)]
#[path = "duplicates.test.rs"]
mod tests;
