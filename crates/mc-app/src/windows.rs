//! The sign-in window, and the passage it opens.
//!
//! ## Why a WINDOW and not a modal
//!
//! Sign-in used to be a dialog over the main window. It worked, and it
//! wasn't right: the main window is one thousand and twenty-four pixels
//! wide because it has an image to show and three sections to carry, and a
//! four-hundred-twenty-pixel modal dropped in the middle of that frame
//! looks like a box forgotten on an empty desk.
//!
//! The design system says it differently, and better: it describes
//! **three windows** — the splash screen at 360 × 480, sign-in at 440 ×
//! 520, the main one at 1024 × 640 minimum. Each is the size of what it
//! carries.
//!
//! ## The path the launcher follows
//!
//! ```text
//!   splash ──┬── session found ──→ main
//!            └── no session   ──→ signin ──→ main
//! ```
//!
//! The main window is CREATED in both cases — it's declared in
//! `tauri.conf.json` with `visible: false` — but it only shows through one
//! of these two paths. That's what lets the front do its network handshake
//! while the player watches the splash screen.
//!
//! ## Closing sign-in QUITS the launcher
//!
//! And that's the only reasonable thing: the main window is hidden, there's
//! nothing behind it, and a process that survives its last visible window
//! is a process you only find again in the task manager. The guard only
//! plays as long as sign-in hasn't succeeded — after that, it's
//! `sign_in_succeeded` that closes the window, and the flag is lowered.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::Error;

/// The label of the sign-in window.
pub const LABEL: &str = "signin";

/// The route the window loads.
///
/// **The application, and not a static page.** A static page was tried: it
/// displayed faster, and it rendered halfway — the Microsoft button had
/// neither the size nor the shape of its siblings, there was neither frosted
/// glass nor an image behind it, because copying six hundred and twenty
/// lines of design system by hand isn't done.
///
/// The delay it was trying to remove was fixed at its real place: it wasn't
/// this window opening that showed, it was the main window showing BEFORE
/// it was ready. See `sign_in_succeeded`.
pub(crate) const ROUTE: &str = "/signin";

/// The design system's dimensions, to the line.
pub(crate) const WIDTH: f64 = 440.0;
pub(crate) const HEIGHT: f64 = 520.0;

/// How long the "signed in" screen stays readable, at minimum.
///
/// Without it, sign-in succeeds and the window disappears in the same
/// frame: which reads as a crash rather than a success.
///
/// **Two seconds**, and that's a pacing choice, not a constraint. The value
/// was first set to a second and a half; in practice, that was short for
/// reading a username, a checkmark, and understanding that sign-in
/// succeeded. It's alone on its line so it can be rejudged by eye.
const SIGNED_IN_FLOOR: Duration = Duration::from_secs(2);

/// After how long we switch without waiting for the main window.
///
/// It announces it's ready through `main_ready`; if it doesn't — front
/// stuck, network hung — it's better to show a window that will catch up
/// than leave the player in front of a "signed in" screen that leads
/// nowhere.
const GUARD_DELAY: Duration = Duration::from_secs(8);

/// The event by which the main window learns it has a session.
///
/// It loaded its front BEFORE sign-in, and its session service therefore
/// carries a null account. Without this signal, it would show on a sign-in
/// page it has no more reason to display, and the launcher would need to be
/// relaunched to get out of it.
pub const SESSION_EVENT: &str = "session-opened";

/// Has the main window finished preparing?
///
/// It receives `session-opened` while still HIDDEN, rereads its session,
/// refreshes the pack state and navigates to Spawn — then says so. Showing
/// it before that would show its sign-in page, then a screen filling up:
/// exactly what we're trying to remove.
static MAIN_READY: AtomicBool = AtomicBool::new(false);

/// Is sign-in in progress?
///
/// Read by `startup::complete`: as long as it's raised, closing the splash
/// screen must NOT show the main window — that would show it empty, on a
/// sign-in page that lives elsewhere, while the player authenticates.
static SIGN_IN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// True as long as the player has no session and the dedicated window is
/// there.
pub fn sign_in_in_progress() -> bool {
    SIGN_IN_IN_PROGRESS.load(Ordering::Acquire)
}

/// Opens the sign-in window, and hides the main one.
///
/// Called by the main window's front, as soon as it knows the session isn't
/// playable. Idempotent: if the window already exists, focus is given back
/// to it rather than creating a second one — two sign-in windows would
/// request two device codes from Microsoft, and only one of the two would
/// succeed.
#[tauri::command]
pub async fn open_sign_in(app: AppHandle) -> Result<(), Error> {
    tracing::info!(
        windows = ?labels(&app),
        "open_sign_in: session missing"
    );
    SIGN_IN_IN_PROGRESS.store(true, Ordering::Release);

    if let Some(main) = app.get_webview_window("main")
        && let Err(error) = main.hide()
    {
        tracing::warn!(error = %error, "the main window could not hide");
    }

    if let Some(already) = app.get_webview_window(LABEL) {
        tracing::debug!("sign-in window already there: giving it focus");
        already.set_focus().map_err(to_error)?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App(ROUTE.into()))
        .title("Sign in")
        .inner_size(WIDTH, HEIGHT)
        .center()
        .decorations(false)
        // Neither resizable nor maximizable: the design system sets both,
        // and the system rule does what the template can't do on its own —
        // hiding the maximize button wouldn't stop it from existing on a
        // double-click on the bar.
        .resizable(false)
        .maximizable(false)
        .minimizable(true)
        .closable(true)
        .background_color(tauri::window::Color(0x16, 0x14, 0x11, 0xff))
        .build()
        .map_err(to_error)?;

    tracing::info!("session absent: sign-in window opened");

    // Closing sign-in quits the launcher: the main window is hidden,
    // there's nothing behind it, and a process that survives its last
    // visible window only turns up in the task manager.
    let quit = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) && sign_in_in_progress() {
            tracing::info!("sign-in window closed without a session: the launcher stops");
            quit.exit(0);
        }
    });

    Ok(())
}

/// The session is open: the main window prepares, then takes over.
///
/// ## The order, and what each step settles
///
/// 1. The sign-in window shows "signed in" — it leads what follows, and the
///    player has proof in front of them that it worked.
/// 2. The main window, still hidden, is TOLD. It rereads its session,
///    refreshes the pack state and navigates to the home screen.
/// 3. The HOME SCREEN says it's drawn — from its `afterNextRender`, so
///    after rendering and not after navigation. The difference isn't
///    theoretical: `navigate` returns once the route is active, which
///    precedes the first pixel.
/// 4. The windows are swapped: the main one shows, sign-in closes.
///
/// The two-second floor runs in parallel with steps 2 and 3: the main
/// window prepares while the "signed in" screen is being read, so the total
/// is the longer of the two, not their sum.
#[tauri::command]
pub async fn sign_in_succeeded(app: AppHandle) -> Result<(), Error> {
    let start = Instant::now();
    tracing::info!(
        windows = ?labels(&app),
        floor_ms = SIGNED_IN_FLOOR.as_millis(),
        "sign_in_succeeded: starting the switch"
    );

    // Lowered first: it's what disarms the closing guard, and the closing
    // happens further below.
    SIGN_IN_IN_PROGRESS.store(false, Ordering::Release);
    MAIN_READY.store(false, Ordering::Release);

    match app.emit_to("main", SESSION_EVENT, ()) {
        Ok(()) => tracing::info!(
            event = SESSION_EVENT,
            target = "main",
            "session signal emitted"
        ),
        Err(error) => {
            tracing::warn!(error = %error, "the main window did not receive the session signal");
        }
    }

    tokio::time::sleep(SIGNED_IN_FLOOR).await;
    tracing::debug!(
        elapsed_ms = start.elapsed().as_millis(),
        main_ready = MAIN_READY.load(Ordering::Acquire),
        "\"signed in\" floor elapsed"
    );

    while !MAIN_READY.load(Ordering::Acquire) && start.elapsed() < GUARD_DELAY {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    if MAIN_READY.load(Ordering::Acquire) {
        tracing::info!(
            wait_ms = start.elapsed().as_millis(),
            "the main window announced it was ready"
        );
    } else {
        tracing::warn!(
            wait_ms = start.elapsed().as_millis(),
            "the main window did not announce it was ready: switching anyway"
        );
    }

    switch(&app);
    tracing::info!(
        total_ms = start.elapsed().as_millis(),
        "sign_in_succeeded: switch done"
    );
    Ok(())
}

/// The main window announces it has something to display.
///
/// Called by its front once the session has been reread and Spawn mounted.
/// A command rather than an event: it's an answer to a question asked, and
/// Rust must be able to wait for it.
#[tauri::command]
pub async fn main_ready(window: tauri::Window) {
    // **The label is logged, and that's the part of the setup that was
    // missing.** This command was called by the SIGN-IN window for several
    // days: it was receiving a signal that wasn't meant for it, navigating
    // to Spawn, and therefore announcing that a window was ready — the
    // wrong one. Nothing in the log made it possible to see that.
    let label = window.label();
    if label != "main" {
        tracing::warn!(
            window = label,
            "\"main_ready\" came from a window OTHER than the main one: ignored"
        );
        return;
    }
    tracing::info!(window = label, "the main window says it's ready");
    MAIN_READY.store(true, Ordering::Release);
}

/// Shows the main window, closes sign-in.
///
/// The order matters. Showing BEFORE closing: the reverse would leave, for
/// one frame, zero visible windows — which some desktop managers treat as
/// an application ending, removing its entry from the taskbar.
fn switch(app: &AppHandle) {
    tracing::info!(windows = ?labels(app), "switch: showing \"main\", closing \"signin\"");

    if let Some(main) = app.get_webview_window("main") {
        if let Err(error) = main.show() {
            tracing::error!(error = %error, "the main window could not show");
        }
        if let Err(error) = main.set_focus() {
            tracing::warn!(error = %error, "focus not given to the main window");
        }
    } else {
        tracing::error!("no \"main\" window to show after sign-in");
    }

    if let Some(window) = app.get_webview_window(LABEL)
        && let Err(error) = window.close()
    {
        tracing::warn!(error = %error, "sign-in window not closed");
    }
}

/// The labels of the live windows, for the log.
///
/// Three lines that exist only for diagnostics, and that earn their place:
/// every defect in this module is a "which one" defect — which one is open,
/// which one spoke, which one showed — and a trace that doesn't name the
/// windows can't settle any of them.
fn labels(app: &AppHandle) -> Vec<String> {
    let mut alive: Vec<String> = app.webview_windows().keys().cloned().collect();
    alive.sort();
    alive
}

/// A Tauri error, in the form the bridge knows how to render.
fn to_error(error: tauri::Error) -> Error {
    Error::from(anyhow::Error::new(error))
}

#[cfg(test)]
#[path = "windows.test.rs"]
mod tests;
