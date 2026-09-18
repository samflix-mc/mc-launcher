//! The files Mojang publishes: client, libraries, assets.
//!
//! Everything starts from `version_manifest_v2.json`, which points to a
//! version's descriptor, which in turn describes the rest. Every file comes
//! with its SHA-1, which makes it possible to verify everything without
//! trusting the transport.
//!
//! The sharing is deliberate: libraries and assets live in a directory
//! common to every instance. They amount to nearly a gigabyte, depend only
//! on the game version, and duplicating them per instance would make having
//! several of them unusable.

mod assets;
mod descriptor;
mod installation;
mod libraries;
mod platform;
mod rules;
mod verification;

pub(crate) use descriptor::{Features, Library, Rule};
pub(crate) use rules::{allowed, allowed_with};

pub use descriptor::Artifact;
pub use installation::{Vanilla, install, java_required};
pub use platform::{maven_path, mojang_arch, mojang_os};
pub use verification::{VerifyReport, classpath, verify_assets};

pub(crate) const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub(crate) const RESOURCES: &str = "https://resources.download.minecraft.net";

/// Concurrent downloads for assets.
///
/// These are a few thousand files of a few kilobytes each: latency
/// dominates, and concurrency is what makes the difference between two
/// minutes and half an hour.
const PARALLEL: usize = 16;
