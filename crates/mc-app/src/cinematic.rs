//! The full sequence, from the window opening to the game launched.
//!
//! Microsoft sign-in, license, pack, loader, game files, Java, NeoForge,
//! mods, lock — then the game.
//!
//! **None of these steps is written here.** `mc-auth` authenticates,
//! `mc-pack::install` installs, `mc-pack::game` prepares and launches. This
//! module calls, aggregates and narrates: that's all the app does.
//!
//! This is deliberate. The command line does the same thing with the same
//! code, and two parallel orchestrations would eventually drift apart —
//! one would install what the other wouldn't launch. Launching itself
//! actually lived for a while in the `mc-pack` binary, out of reach from
//! here; it moved back into the library for that exact reason.
//!
//! ## What the ordering owes to the command line, and what it adds
//!
//! `mc-pack install` authenticates nothing, and `mc-pack launch`
//! authenticates first. Neither checks the license: only `mc-auth login`
//! does, for information. A window can't afford that — making a player wait
//! for eight hundred megabytes only to then tell them their account doesn't
//! own the game would be a mistake. The license is therefore checked before
//! installing anything, and that's the only departure from the CLI.
//!
//! ## In the window, installing and playing are ONE SINGLE gesture
//!
//! `docs/lancement.md` argued the opposite, with a fair point: chaining the
//! two would make anyone who just wanted to play wait for eight hundred
//! megabytes.
//!
//! This module RESOLVES that point instead of contradicting it. Since a
//! digest comparison costs a few dozen kilobytes, the common case — nothing
//! has changed — no longer makes anyone wait. And the case where something
//! has changed is precisely the one where doing nothing would eject the
//! player at sign-in, with no useful message: a player who clicks PLAY on a
//! stale pack didn't choose to play with a stale pack, they chose to play.
//!
//! The command line keeps its two commands, though. `install` and `launch`
//! are tooling use cases, where you want to decide for yourself what
//! happens — and where you're not surprised that a command does what it
//! says.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tauri::{AppHandle, Emitter};

use crate::phase::Phase;
use crate::tracker::Tracker;

/// The event by which progress reaches the window.
pub const EVENT_PROGRESS: &str = "cinematic://progress";

/// Between two snapshots sent to the window.
///
/// Five per second: enough for a rate to look continuous, little enough
/// that the bridge and the render cost nothing. Emitting on every chunk
/// received would send several thousand messages per second for a display
/// that can't show more than sixty.
const CADENCE: Duration = Duration::from_millis(200);

/// The report that `mc-pack` fills, and that the window reads.
///
/// It only writes into the [`Tracker`]: nothing is emitted from here. It's
/// the [`emit_progress`] loop that decides when to speak, and it alone.
struct ToTheWindow {
    tracker: Arc<Tracker>,
}

impl mc_pack::Report for ToTheWindow {
    fn step(&self, step: mc_pack::Step) {
        self.tracker.phase(Phase::from(step));
    }

    fn note(&self, text: &str) {
        // `mc-pack`'s notes are formatted for a terminal and sometimes start
        // with two spaces of indentation. The window places them in its own
        // template.
        self.tracker.note(text.trim());
    }

    fn download(&self, progress: mc_dl::Progress<'_>) {
        self.tracker.download(progress);
    }

    fn resolution(&self, done: usize, total: usize) {
        self.tracker.resolution(done, total);
    }

    /// **The only place that knows the game is running.**
    ///
    /// `mc_pack::Step` stops at the lock: it describes an install, and the
    /// game session isn't part of it. Without this signal, the window used
    /// to stay on "Installing…" for the whole game session — up to three
    /// hours, on a button that announced a download finished long ago.
    fn game_session_started(&self, pid: u32) {
        crate::game_session::started(pid);
        self.tracker.phase(Phase::Launch);
    }
}

/// What the player has set, as `mc-pack` expects it.
///
/// The translation happens HERE and not in `mc-pack`: the install library
/// doesn't depend on `mc-settings`, and must not. It receives values, not a
/// preferences structure.
///
/// A missing or unreadable settings file yields the defaults — never an
/// error: not being able to launch a game session because a comfort file is
/// corrupt would be absurd.
fn player_comfort() -> mc_pack::game::Comfort {
    let settings = mc_settings::load(&mc_settings::path());

    mc_pack::game::Comfort {
        memory_mb: settings.launcher.memory_mb,
        // `Maximized` does NOT go through a resolution: it's up to the game
        // to ask the window manager for the work area, and imposing a size
        // we computed would give a window that covers the desktop panels.
        resolution: match settings.window.mode {
            mc_settings::WindowMode::Windowed => {
                Some((settings.window.width, settings.window.height))
            }
            _ => None,
        },
        fullscreen: settings.window.fullscreen(),
    }
}

/// Where the pack comes from, and where it installs.
///
/// The same settings as the command line with no argument: the pack URL
/// this binary's environment designates, and the default layout. The two
/// must stay identical, otherwise the window would install somewhere other
/// than where `mc-pack launch` goes looking.
fn where_to_install() -> (mc_pack::source::Source, mc_pack::Options) {
    let options = mc_pack::Options::default();
    let source = mc_pack::source::Source::parse(mc_pack::source::default_url(), &options.layout);
    (source, options)
}

/// What's particular about the pack, without installing anything.
///
/// Touches neither disk nor cache: a few dozen kilobytes over the network to
/// know whether the button should say INSTALL or PLAY.
pub async fn pack_state() -> Result<mc_pack::PackState> {
    let (source, options) = where_to_install();
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT).context("HTTP client")?;
    Ok(mc_pack::compare(&source, &options, &dl).await)
}

/// Check, catch up if needed — and **return without playing**.
///
/// The INSTALL button's path. The emission guard is set up first, same as
/// for the other one: it's during the comparison that the window looks
/// frozen.
pub async fn update(app: &AppHandle, tracker: &Arc<Tracker>) -> Result<mc_pack::UpdateOutcome> {
    let (source, options) = where_to_install();

    let _emission = emit_progress(app.clone(), Arc::clone(tracker));

    let reporter: Arc<dyn mc_pack::Report> = Arc::new(ToTheWindow {
        tracker: Arc::clone(tracker),
    });

    let outcome = mc_pack::update(&source, &options, reporter)
        .await
        .context("installing the pack")?;

    tracker.finish(Phase::Ready);
    push(app, tracker);
    Ok(outcome)
}

/// THE gesture: check, catch up if needed, then play.
///
/// ## The emission guard is set FIRST, and that's the point
///
/// It used to live in `install`, and `play` didn't set one up at all. But
/// it's during the COMPARISON that the window looks frozen: a few hundred
/// milliseconds of network during which no step lights up, before it's even
/// known whether there will be an install. Setting it up after the
/// comparison would leave exactly that gap.
pub async fn update_and_play(
    app: &AppHandle,
    tracker: &Arc<Tracker>,
) -> Result<mc_pack::UpdateOutcome> {
    let (source, options) = where_to_install();

    // BEFORE everything else.
    let _emission = emit_progress(app.clone(), Arc::clone(tracker));

    let reporter: Arc<dyn mc_pack::Report> = Arc::new(ToTheWindow {
        tracker: Arc::clone(tracker),
    });

    let outcome = mc_pack::update_and_play(
        &source,
        &options,
        mc_pack::Identity::Microsoft,
        None,
        player_comfort(),
        reporter,
    )
    .await;

    // **Before the `?`, and that's the point.** A process number left behind
    // by a game session that failed would be reused by the system for
    // another program, and the "stop" button would kill that one instead.
    crate::game_session::ended();
    let outcome = outcome.context("launching the game session")?;

    tracker.finish(Phase::Ready);
    push(app, tracker);
    Ok(outcome)
}

/// Checks the files of the installed instance, without downloading anything.
///
/// The "Advanced" section's gesture. It left the main screen with the
/// redesign, and this is where it reappears — without which it would have
/// no graphical entry point left at all.
///
/// `deep` decides what's compared: name and size, or each file's digest.
/// The latter rereads several hundred megabytes, which is why the page asks
/// for it explicitly.
pub fn verify(deep: bool) -> Result<Vec<String>> {
    let (source, options) = where_to_install();
    mc_pack::verify(&source, &options, deep).context("verifying the instance")
}

/// Emits a progress snapshot at a fixed cadence, until it's dropped.
///
/// The returned guard stops the loop when it's dropped. Without this, a
/// failed install would leave a task talking into the void for the rest of
/// the session.
fn emit_progress(app: AppHandle, tracker: Arc<Tracker>) -> Emission {
    let (done, mut stop) = tokio::sync::oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let mut clock = tokio::time::interval(CADENCE);
        loop {
            tokio::select! {
                _ = clock.tick() => push(&app, &tracker),
                _ = &mut stop => break,
            }
        }
    });
    Emission { _done: done }
}

/// As long as it's alive, the window is refreshed.
struct Emission {
    _done: tokio::sync::oneshot::Sender<()>,
}

/// A snapshot, right away.
///
/// Used on changes that can't wait for the next cadence — the install
/// finishing, the launch — so the screen doesn't sit for two tenths of a
/// second on a stale state.
fn push(app: &AppHandle, tracker: &Arc<Tracker>) {
    if let Err(error) = app.emit(EVENT_PROGRESS, tracker.snapshot()) {
        tracing::warn!(error = %error, "progress not delivered to the window");
    }
}

#[cfg(test)]
#[path = "cinematic.test.rs"]
mod tests;
