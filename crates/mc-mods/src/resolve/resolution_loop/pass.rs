//! A pass: resolve the queue, download, read the jars, catch up.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use crate::resolve::catchup::catch_up;
use crate::resolve::inspection::missing_requirements;
use crate::resolve::plan::{Installed, Plan};
use crate::resolve::queue::{Key, ResolutionQueue};
use crate::resolve::{Options, Registry};

use super::descent::download_and_read;
use super::retention::retain;

/// Is one more pass needed?
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Suite {
    /// Some requirements found a supplier: the queue isn't empty.
    Continue,
    /// Nothing is missing anymore, or nothing more can be found.
    Done,
}

/// The state a pass moves forward.
pub(super) struct State<'a> {
    pub chosen: &'a mut BTreeMap<Key, Installed>,
    pub queue: &'a mut ResolutionQueue,
    pub impasses: &'a mut BTreeSet<Key>,
    pub plan: &'a mut Plan,
}

pub(super) async fn one_pass(
    registry: &Registry,
    mc: &str,
    loader: &str,
    options: Options,
    pass: usize,
    state: &mut State<'_>,
) -> Result<Suite> {
    // Breadth-first resolution: declared dependencies join the queue.
    //
    // **This is where most of the time goes, and it used to be silent.**
    // Every request queries one or two APIs, one after another: on a pack of
    // fifty mods, some thirty seconds during which not a single byte comes
    // down — so no bar moves, so the screen looks frozen. We announce after
    // EVERY request settled.
    //
    // The total is an estimate that can grow, since resolving one request
    // can spawn others. That's accepted: a bar that steps back a little
    // reads fine, a motionless bar is indistinguishable from a crash.
    let start = std::time::Instant::now();
    let mut done = 0;
    registry.announce(done, state.queue.remaining());
    while let Some(pending) = state.queue.next() {
        retain(
            registry,
            pending,
            mc,
            loader,
            options,
            state.chosen,
            state.queue,
            state.impasses,
        )
        .await?;
        done += 1;
        registry.announce(done, done + state.queue.remaining());
    }

    // **The duration of THIS loop, not just the one for the download.**
    //
    // The log used to announce only "N jars downloaded and inspected in
    // 382 ms," which suggested the Mods step took half a second when it
    // actually took thirty-six. The other thirty-five are here: API calls,
    // one after another.
    if done > 0 {
        tracing::info!(
            pass,
            requests = done,
            duration_ms = start.elapsed().as_millis(),
            "{done} requests resolved against the APIs in {} ms (pass {pass})",
            start.elapsed().as_millis()
        );
    }

    download_and_read(registry, state.chosen, pass).await?;

    // --- Catch-up: what's still missing ---
    let missing = missing_requirements(state.chosen);
    if missing.is_empty() {
        return Ok(Suite::Done);
    }

    catch_up(
        registry,
        missing,
        mc,
        loader,
        state.impasses,
        state.queue,
        &mut state.plan.unresolved,
    )
    .await?;

    if state.queue.is_empty() {
        return Ok(Suite::Done);
    }
    // A new pass is about to resolve the queue: the gaps already recorded
    // could get filled, so we start again from a clean list.
    state.plan.unresolved.clear();
    Ok(Suite::Continue)
}
