//! Build the command line that starts the game.
//!
//! Minecraft doesn't just launch: it gets *composed*. NeoForge's descriptor
//! only holds a delta and points to its base via `inheritsFrom`; the two must
//! be merged, the libraries valid for this system picked, a classpath
//! assembled, and then some twenty variables substituted into arguments,
//! some of which only appear conditionally.
//!
//! Four points decide whether the game starts or not:
//!
//! - **classpath order** — NeoForge replaces some of Mojang's libraries.
//!   Its version has to come first, otherwise the JVM loads Mojang's and the
//!   loader fails on a missing method;
//! - **the vanilla client** — NeoForge doesn't declare it among its
//!   libraries. It's added to the classpath, and FML is what transforms it
//!   at load time;
//! - **natives** — no need to extract them: Mojang's arguments pass
//!   `org.lwjgl.system.SharedLibraryExtractPath`, and LWJGL 3.3 extracts its
//!   own binaries from the classpath jars. The directory just needs to
//!   exist;
//! - **flags** — `--quickPlayMultiplayer` only exists in the descriptor
//!   behind an `is_quick_play_multiplayer` rule. Ignoring flag rules produces
//!   a command line the game refuses.
mod arguments;
mod classpath;
mod command;
mod descriptor;
mod execution;
mod path;
mod session;
mod variables;

pub use command::Command;
pub use execution::{Outcome, Report, run, run_observe};
pub use session::{LaunchOptions, QuickPlay, Session};

pub use path::build;
