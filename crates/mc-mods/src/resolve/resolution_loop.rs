//! The resolution passes, until nothing is missing anymore.

mod arbitration;
mod derived;
mod descent;
mod pass;
mod retention;

use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

use super::duplicates::deduplicate_by_mod_id;
use super::plan::{Installed, Plan};
use super::queue::{Key, ResolutionQueue};
use super::reason::Reason;
use super::request::Request;
use super::{MAX_PASSES, Options, Registry};

use pass::{State, Suite, one_pass};

/// Resolves, downloads and verifies the whole pack.
pub async fn resolve(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
) -> Result<Plan> {
    resolve_with(registry, requests, mc, loader, Options::default()).await
}

#[tracing::instrument(
    name = "resolution",
    skip(registry, requests, options),
    fields(requests = requests.len(), mc, loader)
)]
pub async fn resolve_with(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
    options: Options,
) -> Result<Plan> {
    // Uniqueness key: a project can only be present once. Two versions of
    // the same mod in `mods` make NeoForge fail to load.
    let mut chosen: BTreeMap<Key, Installed> = BTreeMap::new();
    let mut plan = Plan::default();
    // The projects an implicit requirement tried to claim, without winning.
    // See [`super::reason::implicit_impasse`]: without this memory,
    // catch-up would keep pushing back a request that loses the same
    // arbitration over and over.
    let mut impasses: BTreeSet<Key> = BTreeSet::new();

    // Pass 1: what the manifest asks for, and what the APIs declare.
    let mut queue = ResolutionQueue::default();
    for request in requests {
        queue.push(request.clone(), Reason::Explicit, None);
    }

    let mut state = State {
        chosen: &mut chosen,
        queue: &mut queue,
        impasses: &mut impasses,
        plan: &mut plan,
    };

    for pass in 1..=MAX_PASSES {
        if one_pass(registry, mc, loader, options, pass, &mut state).await? == Suite::Done {
            break;
        }
        if pass == MAX_PASSES {
            bail!(
                "resolution doesn't stabilize after {MAX_PASSES} passes — \
                 circular or missing dependencies"
            );
        }
    }

    // A mod that disappears from the plan must be visible. It used to be
    // that `lock` would return 0 having lost half its manifest, and only
    // mc-content's CI noticed it — after the fact, and elsewhere.
    let discarded = deduplicate_by_mod_id(&mut chosen);
    for eviction in &discarded {
        tracing::warn!(
            discarded = %eviction.discarded,
            kept = %eviction.kept,
            mod_id = %eviction.mod_id,
            "\"{}\" discarded in favor of \"{}\": both declare modId \"{}\"",
            eviction.discarded,
            eviction.kept,
            eviction.mod_id
        );
    }
    // Losing a dependency in favor of a mod that carries the same modId is
    // expected behavior; losing a mod the manifest names is a contradiction
    // of that manifest, and it's up to its author to settle it.
    if let Some(eviction) = discarded.iter().find(|e| e.explicit) {
        bail!(
            "\"{}\" and \"{}\" both declare the modId \"{}\": NeoForge would \
             only load one.\n\
             Both are requested by the manifest — remove one of them.",
            eviction.discarded,
            eviction.kept,
            eviction.mod_id
        );
    }

    plan.mods = chosen.into_values().collect();
    plan.mods
        .sort_by(|a, b| a.candidate.slug.cmp(&b.candidate.slug));
    Ok(plan)
}

#[cfg(test)]
#[path = "resolution_loop.test.rs"]
mod tests;
