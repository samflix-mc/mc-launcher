//! The passage from the splash screen to the window.
//!
//! ## Why TWO windows
//!
//! The main window loads Angular: a module of about a hundred and thirty
//! kilobytes to parse, compile and run before the first pixel. During that
//! time, a WebView paints its own background color and nothing else.
//!
//! `backgroundColor` removed the WHITE from that wait, but not the wait
//! itself: you were watching an empty, dark rectangle. A splash written
//! into `index.html` changes nothing about the problem — it belongs to the
//! same document, and so doesn't display any earlier than it does.
//!
//! The only way to show something DURING that time is a second window,
//! with its own document, tiny and script-free. It displays in a few tens
//! of milliseconds; the main one stays hidden and works.
//!
//! ## Who decides it's over
//!
//! The front, and only it: it's the only one that knows when it has
//! rendered. It calls [`front_ready`] after its first render, plus a short
//! delay — see `app.ts`. Rust can't guess it: `RuntimeRunEvent::Ready` says
//! the window EXISTS, not that its content is painted.
//!
//! ## The two bounds, and they don't serve the same purpose
//!
//! A **floor** ([`MINIMUM_DURATION`]): below it, the splash screen passes
//! too fast to be read, and what was meant to look like an intention looks
//! like a display glitch.
//!
//! A **ceiling** ([`GUARD_DELAY`]): if the front never calls, the main
//! window would stay hidden FOREVER behind a screen with no button to
//! close it, and the only way out would be to kill the process.
//!
//! Both are measured from the same instant — the one where the windows
//! exist — and that's why they live here rather than in the front: the
//! front doesn't know when the splash screen appeared, it only knows when
//! it itself finished.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

/// How long the splash screen stays displayed AT MINIMUM.
///
/// Measured on this machine, the front signals its first render after about
/// five hundred milliseconds. That's short enough that there's no time to
/// read anything: the screen appears and disappears, which reads as a
/// flicker and not as a startup.
///
/// **This is a pacing choice, not a technical constraint.** It's tuned by
/// eye, and it's already moved: a second and a half first, then two. The
/// value therefore stands alone on its line, and the tests express
/// themselves RELATIVE to it rather than in raw numbers — otherwise
/// changing it would force rewriting the suite every time, which ends up
/// discouraging anyone from adjusting it.
const MINIMUM_DURATION: Duration = Duration::from_millis(2000);

/// After how long the window is shown without waiting for the front.
///
/// Ten seconds: well beyond any real startup, including on a slow disk on
/// first launch, and well under what a player would accept watching without
/// understanding anything.
const GUARD_DELAY: Duration = Duration::from_secs(10);

/// The instant the windows were created.
///
/// The origin of both bounds. Set in the `setup` hook, which runs once the
/// windows from `tauri.conf.json` actually exist — that is, within a few
/// milliseconds of the moment the splash screen becomes visible.
static START: OnceLock<Instant> = OnceLock::new();

/// The transition only happens once.
///
/// The guard and the front can arrive together — a slow front at exactly
/// the tenth second. Without this lock, the splash screen would be closed
/// twice, which logs an error for nothing.
static TRANSITION_DONE: AtomicBool = AtomicBool::new(false);

/// How much longer to wait before being allowed to show the window.
///
/// A PURE function, separate from the waiting itself: it's what carries the
/// rule, and so it's what can be tested without a clock or a window.
fn remaining_wait(elapsed: Duration) -> Duration {
    MINIMUM_DURATION.saturating_sub(elapsed)
}

/// Reserves the transition, or says it already happened.
///
/// The flag is taken BEFORE the floor wait, not after. Otherwise, two
/// callers arriving during that wait would both get through it and end up
/// showing the window at the same time.
fn reserve() -> bool {
    !TRANSITION_DONE.swap(true, Ordering::AcqRel)
}

/// Shows the main window and closes the splash screen.
fn complete(app: &AppHandle, why: &str) {
    let mut alive: Vec<String> = app.webview_windows().keys().cloned().collect();
    alive.sort();
    tracing::info!(
        why,
        windows = ?alive,
        sign_in_in_progress = crate::windows::sign_in_in_progress(),
        "splash screen closed"
    );

    // **The main window does NOT show if there's no session.**
    //
    // The front may have found out, while the splash screen was holding,
    // that there isn't one: the sign-in window is then already open, and
    // the main one already hidden. Showing it here would make it appear
    // ON TOP, empty, on a sign-in page that lives elsewhere — that is, two
    // sign-in windows, one of which can't be used.
    //
    // The splash screen is closed anyway: that's its job, and the sign-in
    // window has taken over.
    if !should_show_main(crate::windows::sign_in_in_progress()) {
        tracing::info!("session absent: the sign-in window takes over");
        close_splash(app);
        return;
    }

    // The main window FIRST, the splash screen AFTER.
    //
    // The reverse order would leave, between the two calls, an instant
    // where no window is visible: under some window managers, this passes
    // focus to another application, and the launcher would open behind it.
    if let Some(main) = app.get_webview_window("main") {
        if let Err(error) = main.show() {
            tracing::error!(error = %error, "the main window could not show");
        }
        if let Err(error) = main.set_focus() {
            tracing::warn!(error = %error, "focus not given to the window");
        }
    } else {
        tracing::error!("no \"main\" window to show");
    }

    close_splash(app);
}

/// Is the main window allowed to show?
///
/// **A PURE function, and that's the whole point.** The rule it carries
/// fits in one line, but forgetting it costs two overlapping sign-in
/// windows, one of them unusable — and that defect only shows up on
/// screen, on a machine with no session, after a two-second wait. No
/// suite would have caught it as long as the decision lived in the middle
/// of calls that need a display server.
///
/// It was in fact forgotten once: the guard had been written in the design
/// comment and never in the code.
fn should_show_main(sign_in_in_progress: bool) -> bool {
    !sign_in_in_progress
}

/// Closes the splash screen window, whichever path was taken.
fn close_splash(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash")
        && let Err(error) = splash.close()
    {
        tracing::warn!(error = %error, "splash screen not closed");
    }
}

/// The front has finished rendering.
///
/// Called by `app.ts` after its first render. It's the only reliable
/// signal: Rust knows when the window exists, not when its content is
/// painted.
///
/// Asynchronous because it can WAIT: if the front was faster than the
/// floor, the splash screen is held for the remaining time. The front
/// itself doesn't wait for anything useful from this promise — it ignores
/// it.
#[tauri::command]
pub async fn front_ready(app: AppHandle, window: tauri::Window) {
    // The label says WHO signaled its render. Both windows load the same
    // Angular application: without it, two identical log lines can only be
    // told apart by their timestamp.
    tracing::info!(window = window.label(), "front_ready received");

    if !reserve() {
        tracing::debug!(
            window = window.label(),
            "the transition already happened: signal ignored"
        );
        return;
    }

    // The only instant the page is certainly loaded, so the only one where
    // an `eval` reaches its destination. Absent from the production
    // binary.
    #[cfg(debug_assertions)]
    crate::csp::probe(&app);
    #[cfg(debug_assertions)]
    crate::acceptance::probe(&app);

    let elapsed = START.get().map(Instant::elapsed).unwrap_or_default();
    let remaining = remaining_wait(elapsed);
    if !remaining.is_zero() {
        tracing::debug!(
            rendered_in_ms = elapsed.as_millis(),
            wait_ms = remaining.as_millis(),
            "front ready before the floor: the splash screen is held"
        );
        tokio::time::sleep(remaining).await;
    }

    complete(&app, "the front signaled its first render");
}

/// Arms the delay guard and sets the origin of both bounds.
///
/// To be called in the `setup` hook: it's the first instant the windows
/// actually exist.
pub fn arm_guard(app: &AppHandle) {
    // `set` rather than `get_or_init`: a second set would be a sequencing
    // bug, and ignoring it silently would make us measure the floor from
    // the wrong instant.
    if START.set(Instant::now()).is_err() {
        tracing::warn!("the startup origin was already set");
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(GUARD_DELAY).await;
        if !reserve() {
            return;
        }
        tracing::warn!(
            seconds = GUARD_DELAY.as_secs(),
            "the front signaled nothing: the window is shown anyway. \
             Look for a JavaScript error in the --debug build's console."
        );
        complete(&app, "guard delay elapsed");
    });
}

#[cfg(test)]
#[path = "startup.test.rs"]
mod tests;
