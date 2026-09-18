//! The single gesture, and what the window learns from it.

use serde::Serialize;
use tauri::{AppHandle, State};

use super::{AppState, Error};

/// What a gesture left behind.
///
/// **The same type for both gestures**, and that's deliberate: installing
/// and playing leave exactly the same traces — missing mods, drifts from
/// the lock, a purge, an unreachable remote pack. Only the `verdict`
/// differs, and it's a sentence.
///
/// Twin types would force the screen to carry two display paths to say the
/// same thing, and the one used less would eventually drift without anyone
/// noticing.
///
/// A REPORT and not a string, and that's a requirement from the front:
/// three fields it displays have no other source. `missing` in particular
/// is a resolution result, readable on no disk — it only exists in the lock
/// the installation just wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// What the gesture left, said in one sentence.
    pub verdict: String,
    /// Did an installation take place?
    pub caught_up: bool,
    /// The mods the resolution didn't find. The pack still installs, but
    /// NeoForge will refuse to start if it's missing them.
    pub missing: Vec<String>,
    /// What the installation drifted from the published lock.
    pub drifts: Vec<String>,
    /// The remote pack was unreachable: what was laid down may no longer
    /// match the servers.
    pub offline: bool,
    /// What the purge erased, if there was a purge.
    pub purge: Vec<String>,
}

/// The report for an outcome, with the verdict that matches the gesture.
///
/// Extracted because it's called by BOTH commands: duplicating it would let
/// one of the two lose a field the day one is added, and that kind of
/// oversight only shows up on screen, on a rare case.
fn report(outcome: &mc_pack::UpdateOutcome, verdict: String) -> Report {
    Report {
        verdict,
        caught_up: outcome.installation.is_some(),
        missing: outcome.missing(),
        drifts: outcome.drifts(),
        offline: outcome.state.offline,
        purge: outcome
            .installation
            .as_ref()
            .map(|placement| placement.purge.cleared.clone())
            .unwrap_or_default(),
    }
}

/// What the disk and the published pack say, without installing anything.
///
/// Called when Spawn opens, and it's what decides what the button
/// displays. A few dozen kilobytes of network: the lock alone.
#[tauri::command]
pub async fn pack_state() -> Result<mc_pack::PackState, Error> {
    Ok(crate::cinematic::pack_state().await?)
}

/// Lays down the pack, and **stops there**.
///
/// ## The button no longer launches the game on its own
///
/// It used to: check, catch up, then start Minecraft, in one click. Sam
/// pushed back on it at acceptance — "this launches the game when we just
/// wanted to install the modpack" — and the point stands: laying down eight
/// hundred megabytes and playing are two intentions, and the second doesn't
/// follow from the first.
///
/// What the single gesture had solved isn't lost: the check stays ahead of
/// both paths, and nobody waits for a download to play a pack that's
/// already up to date.
///
/// The token is the same one as [`play`]'s: an installation and a game
/// session would write into the same directories.
#[tauri::command]
pub async fn install(app: AppHandle, state: State<'_, AppState>) -> Result<Report, Error> {
    let Some(_token) = state.reserve() else {
        return Err(Error("an operation is already in progress".to_string()));
    };

    let outcome = crate::cinematic::update(&app, &state.tracker).await?;

    let verdict = if outcome.installation.is_some() {
        "The pack is installed.".to_string()
    } else {
        "The pack was already up to date: nothing to place.".to_string()
    };
    Ok(report(&outcome, verdict))
}

/// THE button, when it says PLAY.
///
/// Checks the published pack, catches up on what moved if need be, then
/// launches the game session. Only returns once it's over.
///
/// The check stays ahead: removing it would let the player in with
/// NeoForge registries that no longer match, which shows up as an ejection
/// at sign-in with no useful message.
///
/// The installation token is taken here, and its reason has changed: it no
/// longer only protects against two concurrent installations, but against
/// two GAME SESSIONS — two `update_and_play` calls would write into the
/// same directories and launch two games on the same instance.
#[tauri::command]
pub async fn play(app: AppHandle, state: State<'_, AppState>) -> Result<Report, Error> {
    let Some(_token) = state.reserve() else {
        return Err(Error("an operation is already in progress".to_string()));
    };

    let outcome = crate::cinematic::update_and_play(&app, &state.tracker).await?;

    let verdict = outcome
        .session_report
        .as_ref()
        .map(super::verdict)
        .unwrap_or_else(|| "Session ended.".to_string());
    Ok(report(&outcome, verdict))
}

/// Verifies the files of the laid-down instance.
///
/// The "Advanced" section's gesture. Returns the list of problems found,
/// empty when everything's fine — and not a boolean: "three files are
/// missing" and "everything's fine" aren't said the same way.
#[tauri::command]
pub async fn verify_files(deep: bool) -> Result<Vec<String>, Error> {
    // On a dedicated executor: the deep verification rereads and rehashes
    // several hundred megabytes, and holding it on the main executor would
    // freeze the loop that refreshes the window.
    tauri::async_runtime::spawn_blocking(move || crate::cinematic::verify(deep))
        .await
        .map_err(|error| Error(format!("verification interrupted: {error}")))?
        .map_err(Error::from)
}
