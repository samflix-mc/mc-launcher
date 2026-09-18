//! What the player configures, and where it lives.
//!
//! ## Who reads this file
//!
//! `mc-app`, and only it. `mc-pack` does NOT depend on this crate: the
//! install library must not know a UI exists, and the CLI keeps its own
//! flags. The application reads the settings and passes the VALUES to
//! `prepare` — not the struct.
//!
//! That's what lets a session be launched from the command line without a
//! settings file written by the window getting mixed in.
//!
//! ## Where
//!
//! In CONFIG and not in data: these are preferences, small and precious,
//! not a rebuildable cache. On Linux, that means erasing the eight hundred
//! megabytes of instances doesn't take them along. Elsewhere, it does — see
//! `docs/authentification.md`.

mod bounds;
mod options_txt;
mod reading;
mod types;

pub use bounds::{FPS, HEIGHT, MEMORY, RENDER, SCALE, SCRIM_FLOOR, SIMULATION, WIDTH};
pub use options_txt::merge;
pub use reading::{load, path, save};
pub use types::{Appearance, Backdrop, Game, Launcher, SCHEMA, Settings, Window, WindowMode};
