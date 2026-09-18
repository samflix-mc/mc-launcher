//! Installing a Minecraft instance: Mojang files, then NeoForge.
//!
//! The disk is laid out like that of multi-instance launchers, for a
//! reason of space: the libraries and assets of a version weigh close to a
//! gigabyte and only depend on the game version. They're shared, and only
//! the data specific to a session — mods, configuration, saves — belongs
//! to the instance.
//!
//! ```text
//! <data>/
//!   shared/                  ← plays the role of a .minecraft
//!     versions/1.21.1/        vanilla descriptor and client
//!     versions/neoforge-21.1.250/
//!     libraries/  assets/
//!   instances/<name>/
//!     minecraft/              game directory: mods, config, saves
//!   runtime/temurin-21/
//! ```
//!
//! The `shared` directory has the shape of a `.minecraft` because the
//! NeoForge installer requires it: it looks there for the vanilla client to
//! patch and drops what it builds there.

mod layout;
mod verification;

#[cfg(test)]
mod fixtures;

pub mod crash;
pub mod launch;
pub mod neoforge;
pub mod vanilla;

pub use layout::{Instance, Layout};
pub use verification::verify;
