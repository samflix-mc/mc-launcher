//! The game session in progress, and the way to end it.
//!
//! ## Why the launcher must be able to kill the game
//!
//! `play` only returns once the session is over. That's what makes it
//! possible to return a report — what the game left behind, the errors
//! noted — but that assumes the session ends. A Minecraft frozen on a
//! loading screen, a graphics driver that stops responding, a window that
//! never displays: the launcher then waits indefinitely, its button stuck
//! on "Playing", and the only way out is the task manager.
//!
//! ## A process number, and not the handle
//!
//! Tokio's `Child` is borrowed by the loop that reads the game's output for
//! the whole session; sharing it would require a lock on `mc-instance`'s
//! hottest path. A number can be copied, and signaling a process doesn't
//! need anything else.
//!
//! The cost is known and accepted: between the moment the number is read
//! and the moment the signal is sent, the process may have ended. The
//! system then returns an error that's logged without being surfaced —
//! there's nothing to announce to someone who wanted to stop a game that's
//! already stopped.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::commands::Error;

/// The game's process number, or zero if there isn't one.
///
/// Zero as absence rather than an `Option` behind a lock: it's written from
/// the installation task and read from the command, and no system assigns
/// process number zero to a process.
static PID: AtomicU32 = AtomicU32::new(0);

/// The game just started.
pub fn started(pid: u32) {
    PID.store(pid, Ordering::Release);
}

/// The session is over, whichever way it happened.
///
/// **Called on ALL exit paths**, errors included: a number left behind
/// would be reused by the system for another program, and the "stop"
/// button would then kill whatever that is.
pub fn ended() {
    PID.store(0, Ordering::Release);
}

/// The running game's process number, if there is one.
pub fn running_pid() -> Option<u32> {
    match PID.load(Ordering::Acquire) {
        0 => None,
        pid => Some(pid),
    }
}

/// Stops the session, if it's still running.
///
/// Without ceremony: this gesture exists for a game that no longer
/// responds, and asking it politely to close is precisely what doesn't
/// work in that case. A player whose game responds doesn't need this
/// button, they have the game's own menu.
#[tauri::command]
pub async fn stop_game() -> Result<(), Error> {
    let Some(pid) = running_pid() else {
        return Err(Error::from(anyhow::anyhow!("no game session in progress")));
    };

    let (program, args) = stop_order(pid);
    tracing::warn!(pid, "game stop requested from the window");

    // `tokio::process` and not `std::process`: this command runs inside a
    // `#[tauri::command] async fn`, on the runtime that also carries the
    // download and the progress events. `kill` returns in microseconds, so
    // the block would be short — but "short" is not "none", and the async
    // API costs nothing here.
    let output = tokio::process::Command::new(program)
        .args(&args)
        .output()
        .await
        .map_err(|error| Error::from(anyhow::Error::new(error)))?;

    if !output.status.success() {
        // The common case is "this process no longer exists": the session
        // ended between the click and the order. Nothing to announce to
        // the player, who wanted exactly that — for it to stop.
        tracing::info!(
            pid,
            code = output.status.code(),
            "the stop order found nothing to kill"
        );
    }

    ended();
    Ok(())
}

/// How to stop a process, depending on the system.
///
/// A PURE function, separate from the execution: it's the only part of
/// this module that can be tested without launching a real game, and it's
/// the one where a typo costs dearly — a wrong flag kills nothing, and you
/// only notice it with a frozen game in front of you.
///
/// `-KILL` and not `-TERM`: the JVM installs handlers for the latter, and a
/// frozen process doesn't run them. On Windows, `/T` takes down child
/// processes, which the JVM creates for its own use.
fn stop_order(pid: u32) -> (&'static str, Vec<String>) {
    if cfg!(windows) {
        (
            "taskkill",
            vec![
                "/PID".to_string(),
                pid.to_string(),
                "/T".to_string(),
                "/F".to_string(),
            ],
        )
    } else {
        ("kill", vec!["-KILL".to_string(), pid.to_string()])
    }
}

#[cfg(test)]
#[path = "game_session.test.rs"]
mod tests;
