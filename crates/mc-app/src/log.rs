//! What the front end tells the log.
//!
//! ## Why the front end writes into Rust's log
//!
//! The window has no inspector in production: `console.log` goes into the
//! void there. But the bugs that matter are SEQUENCE bugs between two
//! windows and a process — "who navigated, when, and in which one" — and
//! they can only be read if both sides write into the same stream, in
//! order, with the same clock.
//!
//! Two separate logs that need to be stitched together by hand are worse
//! than one: the stitching error is precisely the error being sought.
//!
//! ## The label is NOT sent by the front end
//!
//! It's read from the calling window, which Tauri injects. It's the only
//! point in the setup where you can't get the window wrong — and getting
//! the window wrong is exactly the bug this module exists to track.

use tauri::Window;

/// What the front end may ask for, and nothing else.
///
/// A PURE function: it carries the module's only rule, and that's why it's
/// the one being tested. An unknown level maps to `info` rather than losing
/// the line — a diagnostic message that disappears because it was mislabeled
/// is the opposite of what's wanted here.
fn level_of(raw: &str) -> tracing::Level {
    match raw {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    }
}

/// Writes a line from the front end, under its window's label.
///
/// The target is `front`: `RUST_LOG=front=trace` is then enough to see only
/// the front end, and `RUST_LOG=warn,front=trace` to see it alone in the
/// middle of a silent startup.
#[tauri::command]
pub fn log(window: Window, level: String, message: String) {
    let label = window.label().to_string();
    match level_of(&level) {
        tracing::Level::ERROR => {
            tracing::error!(target: "front", window = %label, "{message}")
        }
        tracing::Level::WARN => tracing::warn!(target: "front", window = %label, "{message}"),
        tracing::Level::DEBUG => {
            tracing::debug!(target: "front", window = %label, "{message}")
        }
        tracing::Level::TRACE => {
            tracing::trace!(target: "front", window = %label, "{message}")
        }
        tracing::Level::INFO => tracing::info!(target: "front", window = %label, "{message}"),
    }
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
