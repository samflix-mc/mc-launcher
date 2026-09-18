//! Launch, wait, and surface what happened.
//!
//! This module doesn't decide what's shown to the player: it returns the raw
//! report, and the caller formats it — a line in a terminal, a banner in a
//! window. What it does handle, though, no caller should have to think about:
//! timestamping the launch before launching, and surfacing incidents.

use anyhow::Result;

use super::GameSession;
use super::incidents::{report_game_crash, report_game_error};

/// Launches the game and waits for it to end.
///
/// The returned `Report` carries the verdict and the errors caught during the
/// game session. A non-zero exit code **is not an error from this
/// function**: the game did launch, and it's up to the caller to decide how
/// to announce it. Returning an `Err` here would force the window to
/// reconstruct the exit code from a message.
///
/// Out of scope for mutation tests: it launches Minecraft and waits for it to
/// end. What it draws from that is checked in `mc_instance::launch`.
#[mutants::skip]
pub async fn play(game_session: &GameSession) -> Result<mc_instance::launch::Report> {
    play_announced(game_session, &crate::progress::Silent).await
}

/// The same, but telling anyone listening that the game has started.
///
/// The announcement carries the process ID: see [`Report::game_session_started`].
/// Without it, the window has no way to tell "we're preparing" from "it's
/// running" — two states of the same call, which only returns control at the
/// end of the game session.
#[mutants::skip]
pub async fn play_announced(
    game_session: &GameSession,
    report: &dyn crate::progress::Report,
) -> Result<mc_instance::launch::Report> {
    let GameSession {
        instance,
        lock,
        version_id,
        command,
        ..
    } = game_session;

    // Timestamped before launching: that's what lets us discard the report
    // from a previous game session, which would send us down a false lead.
    let started_at = mc_instance::crash::now();

    let launch_report =
        mc_instance::launch::run_observe(command, &|pid| report.game_session_started(pid)).await?;

    // Surfaced before the verdict: an error that occurred during a game
    // session that ended cleanly counts as much as a crash, and it's the one
    // we'd never see otherwise.
    for error in &launch_report.errors {
        report_game_error(instance, lock, version_id, error, None);
    }

    match launch_report.outcome {
        mc_instance::launch::Outcome::Normal => {
            tracing::info!("Minecraft ended normally");
        }
        // Closing the game is not a crash: reporting it as one would open an
        // incident for every game session ended from the keyboard.
        mc_instance::launch::Outcome::Interrupted { signal } => {
            tracing::info!(signal, "Minecraft was interrupted");
        }
        mc_instance::launch::Outcome::Failed { code } => {
            tracing::warn!(code, "Minecraft stopped on an error");
            report_game_crash(instance, lock, version_id, started_at, code);
        }
    }

    Ok(launch_report)
}

/// Where the player will find the game's logs, to cite when things went
/// wrong.
pub fn logs(game_session: &GameSession) -> std::path::PathBuf {
    game_session.instance.game_dir.join("logs")
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
