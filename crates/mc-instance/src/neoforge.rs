//! Installing NeoForge, through its official installer.
//!
//! NeoForge's version descriptor isn't enough to install the loader: some
//! of the libraries don't exist as such on a Maven repository, they are
//! *produced* during installation by a chain of processing steps — applying
//! binary patches to the vanilla client, splitting the jar, renaming
//! symbols. These steps are jars shipped with the installer, and their
//! sequence changes from one version to the next.
//!
//! Reimplementing them would mean chasing an internal format forever. So we
//! run the published installer instead, with the Java the launcher has just
//! guaranteed. It's idempotent, which makes it safe to rerun.

mod installer;
mod versions;

pub use installer::{install_client, install_server, version_id};
pub use versions::{latest_for, series_for};

pub(crate) const MAVEN: &str = "https://maven.neoforged.net/releases";
