//! Impose on the crates the locations that Tauri knows.
//!
//! ## The problem, and why it doesn't show
//!
//! The repo's crates derive their paths with `std::env::var_os`; Tauri's
//! resolver goes through the `dirs` crate. Both follow the same convention,
//! but not the same fallback rules: a relative `XDG_DATA_HOME`, a missing
//! `HOME`, a roaming Windows profile make them diverge.
//!
//! Two directory trees for the same data means eight hundred megabytes
//! downloaded a second time into a neighboring directory, and a session
//! that can't be found. Nothing would flag it: both halves of the program
//! would work, each on its own side.
//!
//! ## Why this is possible right after `build()`
//!
//! `build()` returns an `App`, which implements `Manager`: `app.path()`
//! answers from that instant, because `register_core_plugins()` installed
//! the `PathResolver` into the managed state INSIDE `build()`
//! (`tauri/src/app.rs:2398`, `path/plugin.rs:240-246`). There's no need to
//! wait for the `setup` hook, nor the event loop.
//!
//! What does NOT exist yet at this point, on the other hand, is the window:
//! the ones from `tauri.conf.json` are built by the free function `setup()`
//! (`app.rs:2520-2535`), called on `RuntimeRunEvent::Ready`
//! (`app.rs:1422-1428`), inside `App::run`. That's why the builder's
//! `setup` hook stays in place.
//!
//! ## What the derivation does, and what it doesn't
//!
//! We take from Tauri the APPLICATION directories — `app_data_dir()` and
//! `app_config_dir()`, which themselves incorporate the identifier — and
//! let `mc-paths` DERIVE the tree from them. It's the only split that holds
//! up: the roots can diverge from one platform to another, that's their
//! job, but the derivation is just `join`s, and the same ones on both
//! sides.
//!
//! Also taking Tauri's `app_log_dir()` would break this symmetry: it lands
//! under `~/Library/Logs` on macOS, so outside of what's enough to delete
//! to start fresh. Logs stay at `<data>/logs`.

use tauri::{App, Manager};

/// Places Tauri's locations for the whole process.
///
/// To call right after `build()`, BEFORE `mc_log::init`: the log opens a
/// file, and it must open it in the right place on the first try.
pub fn place(app: &App) {
    let resolver = app.path();

    // `data_dir()`, `config_dir()` and `temp_dir()` return a `Result`
    // (`Error::UnknownPath`). There's nothing to recover: without a home
    // directory, there's nowhere to install eight hundred megabytes. We
    // then fall back to what the environment says, which won't be better,
    // but at least won't be empty.
    let fallback = mc_paths::from_system();

    let root = |resolved: Result<std::path::PathBuf, tauri::Error>,
                default: &std::path::Path,
                what: &str| {
        match resolved {
            Ok(path) => path,
            Err(error) => {
                // Before `mc_log::init`: `eprintln!` is all we have, and
                // in a graphical app it goes nowhere. That's accepted —
                // the case is a system without a home directory, where
                // nothing will work anyway.
                eprintln!("[paths] {what} not found ({error}), falling back to the environment");
                default.to_path_buf()
            }
        }
    };

    // `app_data_dir()` and `app_config_dir()`, i.e. Tauri's APPLICATION
    // directories: they incorporate `tauri.conf.json`'s `identifier`
    // themselves, and therefore return `<data>/mc.samflix.launcher`.
    //
    // The previous version took the BARE roots and joined a custom segment,
    // `samflix-mc`, to it, to avoid abandoning what was already there. The
    // argument was sound and it was deliberately overturned: a launcher
    // that keeps its stuff somewhere other than where its own framework
    // expects it is a trap that gets rediscovered on every read. The move
    // costs once; the doubt costs on every pass.
    //
    // `mc_paths::SEGMENT` now equals the same identifier: the command line,
    // which has no Tauri resolver and derives from the environment,
    // therefore ends up at the SAME directory by a different path. That's
    // what the comparison below checks on every startup.
    //
    // The temp directory is the exception, because Tauri has no
    // `app_temp_dir`: we join the segment ourselves, which gives the same
    // result.
    let bases = mc_paths::Bases {
        data: root(resolver.app_data_dir(), &fallback.data, "data"),
        config: root(resolver.app_config_dir(), &fallback.config, "config"),
        temporary: match resolver.temp_dir() {
            Ok(path) => path.join(mc_paths::SEGMENT),
            Err(error) => {
                eprintln!("[paths] temp not found ({error}), falling back to the environment");
                fallback.temporary.clone()
            }
        },
    };

    let wanted = mc_paths::from_bases(bases);

    // The comparison, and the only reason it's here: if the two resolvers
    // ever diverge, it should be learned from a log rather than from a
    // player who downloaded the pack twice. The log isn't open yet — that's
    // `mc_log::init` next — so we keep the mismatch to report right after.
    let mismatch = (wanted.data != fallback.data).then(|| {
        format!(
            "Tauri stores data under {} where the environment says {}",
            wanted.data.display(),
            fallback.data.display()
        )
    });

    if let Err(error) = wanted.create() {
        eprintln!("[paths] could not create ({error})");
    }

    if mc_paths::place(wanted).is_err() {
        // A second placement is a sequencing bug: something called
        // `current()` and placed before we did.
        eprintln!("[paths] the locations were already placed");
    }

    if let Some(message) = mismatch {
        // After placement: the trace goes into the log that placement just
        // located, and not into one that would have been opened in the
        // wrong spot.
        MISMATCH.set(message).ok();
    }
}

/// What placement found, to be logged once `mc-log` is open.
static MISMATCH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Logs what placement found. To call after `mc_log::init`.
pub fn log_the_mismatch() {
    if let Some(message) = MISMATCH.get() {
        tracing::warn!(
            "{message} — the two resolvers do not agree; \
             the crates follow Tauri"
        );
    }
}
