//! Helm's interface, the samflix-mc network's launcher.
//!
//! The window. It authenticates the player, installs the pack, verifies it
//! and launches the game — each of these steps already written and tested
//! in one of the root's crates.
//!
//! ## What this crate does, and what it doesn't
//!
//! It contains no launcher logic. Authentication is `mc-auth`'s,
//! installation is `mc-pack`'s, logging is `mc-log`'s, paths are
//! `mc-paths`'. This module only adds a bridge: serializable types, the
//! commands the front end calls, and the order in which all of it starts.
//!
//! That's also why it's outside the mutation scope (see `default-members`
//! in the root Cargo.toml): mutating a wrapper would ask a test to verify a
//! delegation, which only an end-to-end test — with a display server —
//! could do.
//!
//! ## Why the window logs through `mc-log`
//!
//! `tauri-plugin-log` would write tokens as is. `mc-log` redacts them —
//! `refresh_token`, `access_token` — on the console, in the file and before
//! Sentry. A graphical launcher swallows its standard output: the log file
//! is then the only thing a player can attach to a report, and that's
//! exactly the moment a token must not be in it.

mod brand;
mod cinematic;
mod commands;
mod csp;
mod diagnostic;
// The launcher without its window, served over HTTP. Behind a feature not
// enabled by default: `cargo tauri build` doesn't compile it.
mod acceptance;
#[cfg(feature = "dev-server")]
pub mod dev;
mod game_session;
mod log;
mod navigation;
mod paths;
mod phase;
mod startup;
mod tracker;
mod webkit;
mod windows;

/// Mounts the window and returns control when it closes.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // First, before any thread: the call writes the process environment,
    // and that's only safe while nothing else touches it yet.
    let dmabuf_disabled = webkit::configure_rendering();

    // Before the window and before the log: `--diagnostic` answers and
    // exits. This is what CI runs on the binary it just built to know
    // whether it carries the environment it's supposed to — a production
    // binary that thinks it's "development" would go fetch the dev pack and
    // let players onto the dev server. Opening a window to answer that
    // question would require a display server on a runner that doesn't have
    // one.
    if diagnostic::requested(std::env::args()) {
        print!("{}", diagnostic::report(dmabuf_disabled));
        return;
    }

    // THE ORDER OF WHAT FOLLOWS IS THE POINT, and it doesn't read easily:
    //
    // 1. `build()` builds the application WITHOUT opening a window, but
    //    with its path resolver already in place.
    // 2. So the locations are set here — before the log, which must open
    //    its file in the right place on the first try.
    // 3. `mc_log::init` next.
    // 4. `app.run()` last, and it's the one that creates the window.
    //
    // The trap is at step 1: right after `build()`,
    // `get_webview_window("main")` returns `None`. The windows declared in
    // tauri.conf.json are built by the free function `setup()`
    // (app.rs:2520-2535), called on `RuntimeRunEvent::Ready`
    // (app.rs:1422-1428), inside `App::run`. That's why the `.setup()` hook
    // below STAYS: replacing it with a direct call here would do nothing —
    // without a warning — and the window would keep the title frozen in the
    // config file.
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // The navigation allowlist. As a plugin because `on_navigation`
        // doesn't exist on `tauri::Builder`: only on a webview builder — and
        // ours is declared in tauri.conf.json and doesn't exist yet here —
        // or on a plugin builder, whose store is consulted for EVERY
        // webview (manager/webview.rs:596-602).
        .plugin(navigation::plugin())
        // The packaged build's recipe, played by the window. A plugin
        // because the violation collector must be set BEFORE the document,
        // and a webview declared in tauri.conf.json doesn't exist yet here.
        // Absent from the production binary.
        .plugin(acceptance::plugin())
        // The progress counter lives as long as the window: downloads
        // increment it from their tasks, the emission loop reads it, and no
        // command can own it.
        .manage(commands::AppState::default())
        // The `setup` hook does two things, and it's the only place where
        // they're possible: it runs once `tauri.conf.json`'s windows are
        // actually built — which is NOT the case right after `build()`.
        .setup(|app| {
            use tauri::Manager;

            // `tauri.conf.json`'s title is frozen in the file; this one
            // comes from `MC_LAUNCHER_NOM`. Setting it here avoids having
            // two places to change to rename the launcher, one of which
            // gets forgotten.
            if let Some(window) = app.get_webview_window("main")
                && let Err(error) = window.set_title(brand::name())
            {
                tracing::warn!(error = %error, "window title unchanged");
            }

            // Without it, a JavaScript error would leave the main window
            // hidden FOREVER, behind a splash screen with no button to
            // close it.
            startup::arm_guard(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::brand,
            commands::path,
            commands::status,
            commands::sign_in,
            commands::sign_out,
            // The front end says when it has rendered: that's what closes
            // the splash screen and shows the window.
            startup::front_ready,
            // The sign-in window's two moments: opened when the session is
            // missing, closed when it's there.
            windows::open_sign_in,
            windows::sign_in_succeeded,
            windows::main_ready,
            // The front end writes into the SAME log as Rust: sequencing
            // bugs between two windows only show up in a single stream.
            log::log,
            // The single gesture, and what's needed to draw it.
            commands::pack::pack_state,
            commands::pack::install,
            commands::pack::play,
            commands::pack::verify_files,
            // A way to regain control over a game that stopped responding.
            game_session::stop_game,
            // The network's news.
            commands::news::news,
            // The Spawn page's "Server" panel: is it up, and for how many.
            commands::server::server_status,
            // Settings, and what the screen allows.
            commands::settings::settings,
            commands::settings::save_settings,
            commands::settings::screen,
            commands::settings::open_folder,
        ])
        .build(tauri::generate_context!());

    let application = match application {
        Ok(application) => application,
        Err(error) => {
            // A failed `R::new()` — no display server, WebKit unavailable —
            // would otherwise only go to standard error from a graphical
            // application, which nobody reads, and with no log or Sentry
            // since `mc_log::init` hasn't run yet.
            startup_fallback(&error);
            panic!("window startup: {error}");
        }
    };

    // Tauri's resolver is available as soon as `build()` returns: the
    // crates no longer derive anything on their own from here on.
    paths::place(&application);

    // The guard keeps the logging layers open: dropping it here would flush
    // the file's buffered content and cut off Sentry before the window even
    // shows.
    let _log = mc_log::init("helm");

    paths::log_the_mismatch();

    if dmabuf_disabled {
        // A white window under NVIDIA is hard to diagnose; knowing whether
        // the workaround kicked in — or not — is the first thing to check
        // in the log.
        tracing::info!("NVIDIA driver detected, WebKit DMA-BUF rendering disabled");
    }

    // Takes a closure, and returns nothing: the earlier `expect` was about
    // `build`, not about `run`.
    application.run(|_, _| {});
}

/// Writes the cause of a failed startup where logs usually go, since
/// `mc-log` hasn't been able to open yet.
///
/// Without this, the only trace of a machine without a display server would
/// be a panic on standard error that nobody reads — and the player's report
/// would boil down to "it doesn't open".
fn startup_fallback(error: &tauri::Error) {
    use std::io::Write as _;

    eprintln!("[startup] the window could not be built: {error}");

    let logs = mc_paths::from_system().logs;
    if std::fs::create_dir_all(&logs).is_err() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs.join("startup-failed.log"))
    {
        let _ = writeln!(file, "the window could not be built: {error}");
    }
}
