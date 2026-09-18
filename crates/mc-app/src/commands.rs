//! What the interface is allowed to call.
//!
//! Nothing is rewritten here: `mc-auth` authenticates, `mc-pack` installs and
//! launches. This module translates — serializable structures, events, and
//! readable error messages — and sets the few guards a window requires that
//! a terminal doesn't need.

use std::sync::atomic::{AtomicBool, Ordering};

use mc_auth::{Auth, DeviceCode, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

pub mod news;
pub mod pack;
pub mod settings;

use crate::brand::Brand;
use crate::phase::Phase;
use crate::tracker::Tracker;

/// The event that carries the device code to the window.
///
/// `Auth::login` only returns once the player has gone through Microsoft:
/// the code can't be the command's return value, so it has to be pushed
/// during the wait.
pub const DEVICE_CODE_EVENT: &str = "auth://code";

/// What the application keeps between two commands.
#[derive(Default)]
pub struct AppState {
    /// The progress tracker, shared with the downloads.
    pub tracker: std::sync::Arc<Tracker>,
    /// Is an installation already running?
    ///
    /// A button gets clicked twice, and the second installation would write
    /// into the same directories as the first — two concurrent `install`
    /// calls on the same lock is a half-placed pack. The command line
    /// doesn't have this problem: the same command isn't launched twice in
    /// the same process there.
    in_progress: AtomicBool,
}

/// Sets `in_progress` back to `false` no matter what.
///
/// A `?` in the middle of the installation would leave the function without
/// resetting it, and the button would stay off for the rest of the session.
struct Token<'a>(&'a AtomicBool);

impl Drop for Token<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl AppState {
    /// Takes the installation token, or says it's already taken.
    fn reserve(&self) -> Option<Token<'_>> {
        self.in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Token(&self.in_progress))
    }
}

/// The signed-in account, as the window displays it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub username: String,
    pub uuid: String,
    /// False when the account doesn't have a Minecraft Java Edition
    /// license. The sign-in still succeeds — it's a valid Microsoft account
    /// — but no online server will accept it, and there's no point
    /// installing eight hundred megabytes just to learn that afterward.
    pub owns_the_game: bool,
}

impl From<(&Session, bool)> for Account {
    fn from((session, owns_the_game): (&Session, bool)) -> Self {
        Self {
            username: session.profile.name.clone(),
            uuid: session.profile.id.clone(),
            owns_the_game,
        }
    }
}

/// What the player has to type in on Microsoft's side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeView {
    pub code: String,
    /// The page where the code is typed in by hand.
    pub url: String,
    /// The same page, with the code prefilled. This is the one that gets
    /// opened.
    pub direct_url: String,
}

impl From<&DeviceCode> for DeviceCodeView {
    fn from(code: &DeviceCode) -> Self {
        Self {
            code: code.user_code.clone(),
            url: code.verification_uri.clone(),
            direct_url: code.direct_verification_uri.clone(),
        }
    }
}

/// An error, as it crosses the bridge to the window.
///
/// `anyhow::Error` doesn't serialize, and keeping only its last message
/// would lose the context: it's the full chain that tells "Microsoft
/// sign-in" apart from "Microsoft sign-in: the code has expired".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Error(String);

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self(
            error
                .chain()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" : "),
        )
    }
}

/// A phase of the path, as the window draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub phase: Phase,
    pub label: String,
    pub rank: usize,
}

/// The full path, in order.
///
/// Requested once at opening. The interface needs to know it in full
/// **before** anything starts: that's what distinguishes "we're halfway
/// there" from "something is happening".
#[tauri::command]
pub fn path() -> Vec<StepView> {
    Phase::ALL
        .iter()
        .map(|phase| StepView {
            phase: *phase,
            label: phase.label().to_string(),
            rank: phase.rank(),
        })
        .collect()
}

/// Under what name the launcher presents itself.
///
/// Fixed at compile time by `MC_LAUNCHER_NOM`: "samflix-mc" is the network's
/// name today, not a constant of the product.
#[tauri::command]
pub fn brand() -> Brand {
    Brand::current()
}

/// The account already signed in on this machine, if there is one.
///
/// Called when the window opens. Access lazily refreshes the tokens, hence
/// the write-back: without it, the next launch would start from the stale
/// token and would ask for a code again for nothing.
#[tauri::command]
pub async fn status() -> Result<Option<Account>, Error> {
    let Some(state) = mc_auth::load() else {
        return Ok(None);
    };

    let auth = Auth::resume(&state)?;
    let session = auth
        .session()
        .await
        .map_err(|error| error.context("the saved session is no longer valid — sign in again"))?;
    let owns_the_game = auth.owns_game().await?;
    mc_auth::save(&auth.state().await?)?;

    Ok(Some(Account::from((&session, owns_the_game))))
}

/// Opens a Microsoft session, by device code.
///
/// The flow has no password field of its own: Microsoft gives a code, the
/// player authorizes it in their browser, and the call waits there until
/// they've done so — or until the code expires.
#[tauri::command]
pub async fn sign_in(app: AppHandle, state: State<'_, AppState>) -> Result<Account, Error> {
    state.tracker.phase(Phase::SignIn);
    let auth = Auth::login({
        let app = app.clone();
        move |code| announce(&app, code)
    })
    .await?;

    let session = auth.session().await?;

    state.tracker.phase(Phase::License);
    let owns_the_game = auth.owns_game().await?;
    mc_auth::save(&auth.state().await?)?;

    state.tracker.finish(Phase::License);
    tracing::info!(username = %session.profile.name, "Microsoft session opened");
    Ok(Account::from((&session, owns_the_game)))
}

/// Forgets the session.
#[tauri::command]
pub fn sign_out(state: State<'_, AppState>) -> Result<(), Error> {
    mc_auth::erase()?;
    // What's installed stays on disk: it's THAT which the button reads from
    // now on, not a field populated by the current session. A player who
    // signs out then signs back in gets their pack back in place, where the
    // old version offered to reinstall everything.
    state.tracker.phase(Phase::SignIn);
    tracing::info!("session forgotten");
    Ok(())
}

/// What the game left behind when it stopped, said in one sentence.
///
/// Closing its window isn't a crash, and neither is a signal: only a
/// non-zero exit code is one. Confusing them would flash a red banner at
/// the end of every game session.
pub(crate) fn verdict(report: &mc_instance::launch::Report) -> String {
    match &report.outcome {
        mc_instance::launch::Outcome::Normal => "Session ended.".to_string(),
        mc_instance::launch::Outcome::Interrupted { .. } => "Game closed.".to_string(),
        mc_instance::launch::Outcome::Failed { code } => {
            let detail = report
                .errors
                .first()
                .map(|error| format!(" — {}", error.exception))
                .unwrap_or_default();
            format!("The game stopped on an error (code {code}){detail}")
        }
    }
}

/// Pushes the code to the window, and opens the page.
///
/// The two are attempted separately: a browser that fails to open still
/// leaves the code displayed, which is enough to sign in by hand. The
/// reverse wouldn't be true, hence the order.
fn announce(app: &AppHandle, code: &DeviceCode) {
    let code = DeviceCodeView::from(code);

    if let Err(error) = app.emit(DEVICE_CODE_EVENT, code.clone()) {
        tracing::warn!(error = %error, "device code did not reach the window");
    }

    if let Err(error) = app.opener().open_url(&code.direct_url, None::<&str>) {
        tracing::warn!(error = %error, "Microsoft page not opened, the code stays displayed");
    }
}

#[cfg(test)]
#[path = "commands.test.rs"]
mod tests;
