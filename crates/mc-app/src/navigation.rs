//! Where the window is allowed to go.
//!
//! ## What this prevents
//!
//! The launcher displays news written elsewhere, and a single link would be
//! enough to make the whole window navigate to a remote site. This window
//! isn't a browser: it's a privileged origin where `invoke` is reachable,
//! with access to the file system, the keyring and the player's token. A
//! remote site loaded there would inherit all of that.
//!
//! Links to the outside therefore go through `ouvrirPage()`, which hands
//! them to the system browser — a separate process that knows nothing of
//! the launcher.
//!
//! ## Why a plugin, and not something else
//!
//! `on_navigation` doesn't exist on `tauri::Builder`. It exists on two
//! things only: a webview builder, and a plugin builder
//! (`tauri/src/plugin.rs:458`).
//!
//! The first doesn't fit: our window is declared in `tauri.conf.json` and
//! doesn't exist yet at the point where the application would be built.
//! Rebuilding it in code just to attach a handler would be wrong twice —
//! it doesn't exist after `build()`, and it's pointless, since the plugin
//! store is consulted for EVERY webview, including the one from the
//! configuration file (`manager/webview.rs:596-602`).
//!
//! ## The danger of this control
//!
//! Too strict, it shows a window that's blank WITHOUT A WORD: the initial
//! page itself goes through this handler. That's why every refusal is
//! logged with its URL — a silent refusal is diagnosed by opening the
//! code, a logged refusal is diagnosed by reading the log.

// `tauri::Url` is a re-export of `url::Url` (tauri/src/lib.rs:83). Going
// through it rather than adding the dependency means the type is then
// literally the one `on_navigation` receives, and not a namesake from a
// different version that wouldn't unify with it.
use tauri::Url;

/// The origins that are the application itself.
///
/// Three, not one: the asset protocol's origin isn't the same everywhere.
/// `tauri://localhost` on Linux and macOS, `http://tauri.localhost` on
/// Windows, and `http://localhost:1420` during a `tauri dev`, where it's
/// Angular's server that serves the page.
///
/// Forgetting one gives a blank window on the forgotten platform only —
/// that is, in practice, on someone else's machine.
const ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
    "http://localhost:1420",
];

/// Is navigating to this URL one the application allows?
///
/// A pure function, and that's the whole point: it's tested without a
/// window, without a display server and without Tauri.
pub fn allowed(url: &Url) -> bool {
    let origin = url.origin().ascii_serialization();

    // `origin()` of a `tauri://localhost` URL returns "null": the scheme
    // isn't special in the sense of the URL standard. So the textual form,
    // truncated of its path, is also compared.
    if ORIGINS.contains(&origin.as_str()) {
        return true;
    }

    match url.host_str() {
        Some(host) => {
            let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
            let recomposed = format!("{}://{host}{port}", url.scheme());
            ORIGINS.contains(&recomposed.as_str())
        }
        None => false,
    }
}

/// The plugin that hooks the predicate onto every webview of the
/// application.
pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation")
        .on_navigation(|_window, url| {
            let allowed = allowed(url);
            if !allowed {
                // The only thing that tells apart "the link was blocked,
                // that's normal" from "the window is blank and I don't
                // know why".
                tracing::warn!(url = %url, "navigation refused: outside the application");
            }
            allowed
        })
        .build()
}

#[cfg(test)]
#[path = "navigation.test.rs"]
mod tests;
