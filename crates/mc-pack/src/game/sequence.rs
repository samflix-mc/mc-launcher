//! Check, catch up if needed — and play, or not.
//!
//! ## Two entry points, and why there are two
//!
//! [`update`] lays down what needs laying down and returns control.
//! [`update_and_play`] follows through into the game session. The two share
//! their first half, and that's deliberate: the check comes BEFORE anything
//! else in both cases — launching first and checking afterward would let the
//! player in with NeoForge registries that no longer match.
//!
//! There was only one entry point for a while, the one that follows through,
//! under the name "single gesture". Sam reworked it at acceptance: clicking
//! to lay down a modpack and watching Minecraft start is not what was asked
//! for. Laying down eight hundred megabytes and playing are two intentions.
//!
//! What the single gesture had solved isn't lost: the comparison costs a few
//! dozen kilobytes, the common case — nothing moved — doesn't make anyone
//! wait, and the case where something moved is precisely the one where doing
//! nothing would give an ejection at connection with no useful message.
//!
//! The CLI keeps its two commands: `install` and `launch` are toolsmith
//! usages, where you want to decide for yourself what happens.
//!
//! ## The name
//!
//! Not `join`: that verb is already taken by the target server
//! (`game::target::choose`, `QuickPlay::Multiplayer`, `GameSession.target`),
//! and two meanings for one word in the same crate cost something at every
//! reread.

use std::sync::Arc;

use anyhow::Result;

use crate::comparison::PackState;
use crate::progress::Report;
use crate::source::Source;
use crate::{Options, Outcome};

/// What actually happened.
///
/// Returns a REPORT and not a string, and that's a requirement from the
/// front: three fields it displays have no other source. `drifts` and
/// `offline` come from the pack's state; `missing` is a resolution result,
/// readable on no disk — it only exists in the lock the installation just
/// wrote.
///
/// A string would force the window to reparse a message to pull out a list,
/// which is exactly what we don't want from a bridge.
#[derive(Debug)]
pub struct UpdateOutcome {
    /// What the comparison saw before acting.
    pub state: PackState,
    /// The installation, if it took place.
    pub installation: Option<Outcome>,
    /// The game session's report, if the game was launched.
    pub session_report: Option<mc_instance::launch::Report>,
}

impl UpdateOutcome {
    /// The dependencies no source could provide.
    ///
    /// Empty when nothing was installed — we don't claim to know what a
    /// resolution that never happened would have found.
    pub fn missing(&self) -> Vec<String> {
        self.installation
            .as_ref()
            .map(|placement| {
                placement
                    .lock
                    .unresolved
                    .iter()
                    .map(|missing| missing.mod_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// What the installation drifted from what was expected.
    pub fn drifts(&self) -> Vec<String> {
        self.installation
            .as_ref()
            .map(|placement| placement.drifts.clone())
            .unwrap_or_default()
    }
}

/// Checks, and catches up if needed. **Without launching the game.**
///
/// ## Why the two gestures are split apart again
///
/// The single gesture had one button check, catch up, and launch. In
/// practice, Sam reworked it on that point, and the pattern is clear:
/// clicking to lay down a modpack and watching Minecraft start on its own is
/// not what was asked for. Laying down eight hundred megabytes and playing
/// are two intentions, and the second doesn't follow from the first.
///
/// What the single gesture had solved isn't lost either: the check stays
/// ahead of BOTH paths, it still costs a few dozen kilobytes, and nobody
/// waits for a download to play a pack that's already up to date.
pub async fn update(
    source: &Source,
    options: &Options,
    report: Arc<dyn Report>,
) -> Result<UpdateOutcome> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    // It's during the comparison that the window looks frozen: a few hundred
    // milliseconds of network with no step lighting up.
    report.note("Checking the pack…");
    let state = crate::comparison::compare(source, options, &dl).await;

    let installation = if should_catch_up(&state) {
        tracing::info!(drift = ?state.drift, "catching up the pack");
        Some(crate::install(source, options, Arc::clone(&report)).await?)
    } else {
        tracing::info!(drift = ?state.drift, "nothing to catch up");
        None
    };

    Ok(UpdateOutcome {
        state,
        installation,
        session_report: None,
    })
}

/// Checks, catches up if needed, then launches the game session.
///
/// The order isn't negotiable: the comparison decides, and it comes before
/// everything. Launching first and checking afterward would let the player
/// in with NeoForge registries that no longer match.
#[allow(clippy::too_many_arguments)]
pub async fn update_and_play(
    source: &Source,
    options: &Options,
    identity: super::Identity,
    server: Option<String>,
    comfort: super::Comfort,
    report: Arc<dyn Report>,
) -> Result<UpdateOutcome> {
    let UpdateOutcome {
        state,
        installation,
        ..
    } = update(source, options, Arc::clone(&report)).await?;

    let game_session = super::prepare(
        source,
        options,
        identity,
        server,
        comfort,
        Arc::clone(&report),
    )
    .await?;
    let session_report = super::play_announced(&game_session, report.as_ref()).await?;

    Ok(UpdateOutcome {
        state,
        installation,
        session_report: Some(session_report),
    })
}

/// Should we install before playing?
///
/// A PURE function, extracted from the body so it can be tested: the body
/// itself launches Minecraft and waits for it to end.
///
/// Offline, we catch up on NOTHING, and that's the point that's hard to
/// guess. There's nothing to decide from: `Drift::Unknown` doesn't mean
/// "up to date", it means "we don't know". Installing on that basis would
/// start over from the cache to lay down what's already there — several
/// minutes of checksum verification, learning nothing, at the exact moment
/// the player has no network and just wants to play.
pub fn should_catch_up(state: &PackState) -> bool {
    crate::comparison::should_place(state.drift, state.offline)
}

#[cfg(test)]
#[path = "sequence.test.rs"]
mod tests;
