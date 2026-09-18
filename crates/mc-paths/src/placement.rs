//! How the application imposes its locations on the crates.
//!
//! ## The problem this solves
//!
//! Each crate derived its path on its own, by re-reading the environment.
//! The application, though, has a resolver — Tauri's — that doesn't use the
//! same rules: it goes through the `dirs` crate, where the repo's crates
//! read `std::env::var_os`. Three known cases make them diverge: a relative
//! `XDG_DATA_HOME`, a missing `HOME`, a roaming Windows profile.
//!
//! Two different locations for the same data means eight hundred megabytes
//! downloaded twice, and a session that can't be found again.
//!
//! ## Why a placement, and not an argument
//!
//! Threading the locations by hand all the way down to `mc-auth` would have
//! required changing the signature of some thirty functions, several of
//! which are a crate's public interface. A single placement, done at startup
//! before everything else, costs one `OnceLock` and changes no signature.
//!
//! The price is named: it's a global state, and so it is placed ONCE, never
//! replaced, and never read before the placement in the normal path.

use std::sync::OnceLock;

use crate::Locations;

/// The placement has already happened.
///
/// A type and not a `bool`: the only legitimate caller is the application's
/// startup, and a second placement is a sequencing bug that must be seen,
/// not a situation to recover from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyPlaced;

impl std::fmt::Display for AlreadyPlaced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the locations have already been placed; they cannot be replaced while running"
        )
    }
}

impl std::error::Error for AlreadyPlaced {}

static PLACED: OnceLock<Locations> = OnceLock::new();

/// Imposes the locations, once for the whole life of the process.
///
/// Must be called BEFORE the first `current()`, i.e. before the log is
/// opened and before any command. A late placement would replace nothing and
/// would leave two halves of the program on two different trees.
pub fn place(locations: Locations) -> Result<(), AlreadyPlaced> {
    PLACED.set(locations).map_err(|_| AlreadyPlaced)
}

/// The locations currently in effect.
///
/// ## The fallback, and why it doesn't memoize
///
/// Without a placement, [`crate::from_system`] is recomputed ON EVERY CALL.
/// This isn't an optimization that got missed: it's today's behavior, the
/// one seven test suites rely on mid-process — including
/// `mc-pack/src/commands/arguments.test.rs`, through `Options::default()`. A
/// MEMOIZED fallback would freeze the very first caller's environment, and a
/// test that moves `XDG_DATA_HOME` would see it or not depending on its rank
/// in the suite.
///
/// The cost is a few `join`s: this path isn't hot.
pub fn current() -> Locations {
    match PLACED.get() {
        Some(locations) => locations.clone(),
        None => crate::from_system(),
    }
}

/// Have the locations been imposed?
///
/// For diagnostics only: "what Tauri gave" or "what the environment says"
/// isn't the same answer to the same question, and it's the first thing to
/// know when a player can't find their data.
pub fn placed() -> bool {
    PLACED.get().is_some()
}

#[cfg(test)]
#[path = "placement.test.rs"]
mod tests;
