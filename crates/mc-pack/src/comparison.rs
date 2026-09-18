//! "Is there anything to do before playing?"
//!
//! That's the question the single button asks on every click, and it's the
//! only thing this module knows how to do.
//!
//! ## Why a separate module, and why not `load_remote`
//!
//! The only existing path that goes and fetches the remote pack is
//! `load_remote`: it fetches the manifest AND the lock together, then
//! **saves them to the cache** before the slightest installation has begun
//! (`source/remote/fetch.rs:21-45`).
//!
//! Reusing it here would be a mistake. The cache is the description of the
//! **placed** pack, the one that `game::coherence::verify` relies on; writing
//! the **published** pack there before installing anything would make the
//! rest of the program believe it already has what it has only just looked
//! at. The offline verification that would follow would then compare the
//! disk to a lock that nothing has installed.
//!
//! Hence a dedicated function, which reads and keeps nothing — and a test
//! that asserts the cache directory is **unchanged** after a comparison.
//! That's the only way to hold this boundary over time: it doesn't show in a
//! signature.
//!
//! ## The single gesture
//!
//! `docs/lancement.md` used to state "installing and playing remain two
//! gestures". This module reverses the decision, and resolves its underlying
//! motive instead of contradicting it: the motive was that installing is
//! expensive and must not be triggered by surprise. With a digest
//! comparison, PLAY costs nothing when there's nothing to do — and when
//! there's something to do, not doing it would cause an ejection at connect.
//!
//! The CLI keeps its two commands: those are tooling-operator use cases.

use anyhow::Result;
use serde::Serialize;

use crate::lockfile::Lockfile;
use crate::source::Source;
use crate::state::LocalState;

/// What the button must say.
///
/// Two values, not three: ACTIVITY — an installation in progress, a session
/// running — isn't derived from disk, it's observed. Conflating it with the
/// action would make the button answer "an installation is already in
/// progress" to someone who clicks during their session.
///
/// An enum without `Default`: that's what makes return-value mutants
/// unviable. A `String` or a `bool` would have let `String::new()` and
/// `false` slip through without any test flinching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Nothing is placed: the gesture is a first installation.
    Install,
    /// Something is placed: the gesture is to play, catching up if needed.
    Play,
}

/// What separates the placed pack from the published one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Drift {
    /// Nothing is installed on this machine.
    Absent,
    /// The placed pack matches the published one: playing will download
    /// nothing.
    UpToDate,
    /// The published pack has moved: a few files to catch up on.
    Update,
    /// The pack demands a full reinstall.
    Reinstall,
    /// Unknown — the published pack is unreachable.
    ///
    /// **This is not an error.** An offline player whose pack is coherent
    /// must be able to play: the button says PLAY, and the installation that
    /// follows will fall back to the cache.
    Unknown,
}

/// What the front end needs to know to draw the screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackState {
    pub action: Action,
    pub drift: Drift,
    /// The published pack didn't respond.
    pub offline: bool,
    /// Something is placed on this disk.
    pub installed: bool,
    /// The pack's name, as carried by the lock.
    pub name: Option<String>,
    /// Its version, when the pack declares one.
    pub version: Option<String>,
    /// The major Java version the lock REQUIRES.
    ///
    /// Read from the lock and not from an installation: the Settings page
    /// must be able to display it even when nothing is installed yet.
    pub java: Option<u32>,
    /// How many mods the published pack counts.
    pub mods: usize,
    /// The generation demanded by the published pack.
    pub generation: u32,
}

/// What's placed on this disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    /// The instance exists and the launcher has kept what it put there.
    pub installed: bool,
    pub state: Option<LocalState>,
}

/// Reads the disk. **Never returns an error.**
///
/// A disk that can't be read means "nothing installed", not "can't play".
/// The distinction wouldn't give the caller anything: in both cases the
/// right move is to install.
///
/// Fixes the repo's third piece of debt: `State.installation` used to be
/// populated only by an installation done in the current session, so on
/// startup a perfectly installed pack looked absent.
pub fn presence(options: &crate::Options, pack_name: &str) -> Presence {
    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(pack_name));

    let state = LocalState::read(&crate::state::path(&instance));

    // Both, not either. An `state.json` without a mods directory describes an
    // installation someone half-erased by hand; a mods directory without a
    // state file is an installation from before this version, about which
    // nothing is known.
    let installed = state.is_some() && instance.mods_dir().is_dir();

    Presence { installed, state }
}

/// Goes and fetches the published lock, and **writes nothing**.
///
/// No cache, no temp file, no trace. See the module header for why — it
/// isn't guessable from this signature, and that's exactly why a test guards
/// it.
pub async fn published_lock(url: &str, dl: &mc_dl::Downloader) -> Result<Lockfile> {
    let address = crate::source::lock_url_for(url);
    let bytes = dl.bytes(&address).await?;
    Lockfile::parse(&bytes)
}

/// Compares what's placed to what's published.
///
/// The core of the single button. Only downloads the lock — a few tens of
/// kilobytes — and doesn't touch the disk.
pub async fn compare(
    source: &Source,
    options: &crate::Options,
    dl: &mc_dl::Downloader,
) -> PackState {
    let url = match source {
        Source::Remote { url, .. } => url.clone(),
        // A local pack has no "published" counterpart: it's the file being
        // edited that's authoritative. We don't claim to know whether
        // there's an update, and the button goes by presence alone.
        Source::File { .. } => return without_network(options, "local pack"),
    };

    let published = match published_lock(&url, dl).await {
        Ok(lock) => lock,
        Err(error) => {
            tracing::warn!(url, error = %error, "published lock unreachable");
            return without_network(options, &url);
        }
    };

    let placed = presence(options, &published.name);
    let published_digest = published.digest().ok();

    let drift = match (&placed.state, published_digest) {
        // **Nothing on disk: first installation, whatever the witness says.**
        //
        // `state.json` and the instance don't die together: an `rm -rf` on
        // `instances/<pack>/minecraft/` leaves the witness intact, one level
        // up the tree. We used to compare the placed lock's digest to the
        // published one, find them equal, and conclude "up to date" — on an
        // empty directory.
        //
        // The button did say INSTALL, because its `Action` looks at
        // presence; but the installation itself reads the DRIFT, and so had
        // nothing to do. Clicking reported "the pack was already up to
        // date: nothing to place", and before the gestures were split, the
        // game would launch on an instance without mods.
        _ if !placed.installed => Drift::Absent,
        // No witness: first installation.
        (None, _) => Drift::Absent,
        // The lock's digest can't be computed — a case that shouldn't
        // happen, serializing a structure just read. We don't guess: we'll
        // catch up by diffing, which only costs a digest walk.
        (Some(_), None) => Drift::Update,
        (Some(state), Some(digest)) => {
            if state.generation < published.generation {
                // The purge wins over everything: it's an explicit request
                // from whoever publishes, and it holds even when the locks
                // are otherwise identical.
                Drift::Reinstall
            } else if state.lock_sha512 == digest {
                Drift::UpToDate
            } else {
                Drift::Update
            }
        }
    };

    PackState {
        action: action_for(drift, false, placed.installed),
        drift,
        offline: false,
        installed: placed.installed,
        name: Some(published.name.clone()),
        version: published.version.clone(),
        java: Some(published.java),
        mods: published.mods.len(),
        generation: published.generation,
    }
}

/// Is there anything to place on this disk?
///
/// **The rule lives here, and only here**: `game::should_catch_up` calls it,
/// and the button reads it through [`Action`]. Two copies would diverge the
/// day a new drift is added, and the symptom would be a button offering to
/// play a pack it just decided to reinstall.
///
/// Offline, we place NOTHING, and that's the part that's easy to miss:
/// `Drift::Unknown` doesn't mean "up to date", it means "we don't know".
/// Installing on that basis would fall back to the cache to re-place what's
/// already there — several minutes of digest verification, learning nothing,
/// at the exact moment the player has no network.
pub fn should_place(drift: Drift, offline: bool) -> bool {
    if offline {
        return false;
    }
    match drift {
        Drift::Absent | Drift::Update | Drift::Reinstall => true,
        Drift::UpToDate | Drift::Unknown => false,
    }
}

/// What the button must DO.
///
/// ## The button no longer launches the game by itself
///
/// It used to say "Update and play", and it did both. Sam pushed back on
/// this during acceptance testing: clicking to place a modpack and seeing
/// Minecraft start isn't what was asked for. As soon as there's something to
/// place, the gesture is to PLACE — playing comes after, on a second click,
/// when the player decides.
///
/// `!installed` stays in the rule, and it isn't redundant: offline,
/// `should_place` returns false even when nothing is placed, and without
/// this term the button would say PLAY to someone with no game to launch.
fn action_for(drift: Drift, offline: bool, installed: bool) -> Action {
    if !installed || should_place(drift, offline) {
        Action::Install
    } else {
        Action::Play
    }
}

/// What's returned when the published pack is out of reach.
///
/// **Not an error.** The button says PLAY if something is placed, and the
/// installation that follows will fall back to the cache. Refusing to play
/// because verification failed would punish a player for a network outage.
fn without_network(options: &crate::Options, reason: &str) -> PackState {
    // We don't know the published pack's name: we query the disk with the
    // configured instance name, and failing that, the network's default
    // name.
    let placed = presence(options, crate::state::DEFAULT_NAME);
    tracing::info!(
        reason,
        installed = placed.installed,
        "comparison without network"
    );

    PackState {
        action: action_for(Drift::Unknown, true, placed.installed),
        drift: Drift::Unknown,
        offline: true,
        installed: placed.installed,
        name: None,
        version: None,
        java: None,
        mods: 0,
        generation: placed.state.map(|e| e.generation).unwrap_or(0),
    }
}

#[cfg(test)]
#[path = "comparison.test.rs"]
mod tests;
