//! Where the launcher keeps its things — one single place that decides.
//!
//! ## Why this crate exists
//!
//! Before it, four places derived a path, each its own way: `mc-dl` for the
//! data, `mc-auth` for the session, `mc-log` for the logs, `mc-instance` for
//! the instance root. Three of them read the environment, each with its own
//! fallback rules, and nothing guaranteed they'd agree.
//!
//! Worse, the graphical application had no way to impose its own on them.
//! Tauri exposes a resolver that knows each system's conventions — and it
//! isn't this one: it goes through the `dirs` crate, where the repo's crates
//! read `std::env::var_os`. Three known cases make them diverge, and the
//! symptom would be eight hundred megabytes downloaded a second time into a
//! neighboring directory.
//!
//! ## What the crate separates
//!
//! Two things the earlier version conflated:
//!
//! - **Choosing the roots** — [`from_system`] — reads the environment, has a
//!   branch per platform, and CAN diverge from what Tauri finds.
//! - **Deriving the tree** — [`from_bases`] — is nothing but `join`s, and
//!   CANNOT diverge. It's the one the application uses on Tauri's roots.
//!
//! Comparing the two, logged at `warn` when they don't match, is what will
//! make it possible to know there's a problem before a player reports it.
//!
//! ## A leaf, and it matters
//!
//! No dependency outside `std`. That's what lets `mc-dl` — which everything
//! else depends on — depend on this one without creating a cycle, and what
//! makes its coverage fully reachable through pure tests.

mod locations;
mod placement;

pub use locations::{
    Bases, Locations, SEGMENT, bases_linux, bases_macos, bases_windows, from_bases, from_system,
};
pub use placement::{AlreadyPlaced, current, place, placed};
