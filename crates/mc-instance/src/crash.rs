//! What Minecraft leaves behind when it stops badly.
//!
//! A player whose game crashes can't read a Java trace and won't think to
//! attach a file. The launcher, though, knows exactly where to look:
//!
//! - `crash-reports/crash-*.txt` — written by the game when it catches
//!   the exception. The richest source: description, trace, loaded mods,
//!   graphics driver;
//! - `logs/latest.log` — the rest of the time. A mod loading error or a
//!   module conflict shows up there, while no report is produced because
//!   the JVM stops before the game exists.
//!
//! The second case is the most common with a modpack, and it's exactly the
//! one no crash report covers.

mod reading;
mod report;
mod watch;

pub use reading::{Crash, parse};
pub use report::{find, now};
pub use watch::{Watcher, loaded_mods};
