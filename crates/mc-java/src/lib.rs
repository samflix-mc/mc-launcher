//! Detect the EXACT Java runtime the pack requires, otherwise install a
//! Temurin.
//!
//! Minecraft 1.21.1 requires Java 21: below that, the game stops on
//! `UnsupportedClassVersionError` before even showing a window. A player has
//! no reason to have a JDK, and the one they have is often an 8 or a 17 left
//! over by some old modpack.
//!
//! ## "Exactly", not "at least"
//!
//! The major version demanded is the one the LOCK carries — so the one
//! NeoForge was installed with. A newer Java isn't "good enough": it changes
//! mixin behavior and the registry format, and the server cuts it off with an
//! ejection that doesn't name its cause.
//!
//! The launcher therefore checks for equality, and does so on every launch. A
//! machine with a Java 22 and a pack that asks for 21 will get a dedicated
//! Temurin 21, without touching the system's 22.
//!
//! The rule holds at BOTH ends — [`detect`] and installation — and this isn't
//! decorative symmetry: without both, installation would accept what
//! detection refuses, and `ensure` would reinstall a hundred and eighty
//! megabytes on every launch without ever converging.
//!
//! The strategy is therefore: search, verify, and only install as a last
//! resort — installing unconditionally would cost 50 MB on every machine for
//! nothing, and never installing would send the user to a download page,
//! which is exactly what a launcher should avoid.
//!
//! The installed runtime is **dedicated to the launcher**: it lives in its
//! data directory, isn't added to `PATH`, and doesn't touch the system's Java.
mod adoptium;
mod archive;
mod detection;
mod installation;
mod locations;
mod version;

pub use detection::{detect, ensure};
pub use installation::install;
pub use locations::{default_runtime_dir, managed_home};
pub use version::{Java, Origin, Version, parse_major, probe};

#[cfg(test)]
mod fixtures;
