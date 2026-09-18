//! Where this binary comes from, and therefore where to file its incidents.
//!
//! An environment describes a **deployment**, not a build profile. Mixing
//! the two up has an immediate consequence: a `cargo run --release` on a
//! development machine doesn't have `debug_assertions`, and ends up filed
//! as production. The project's first test incidents arrived this way, in
//! an environment where nothing had ever been deployed.
//!
//! The environment is therefore **declared**, never inferred:
//!
//! 1. `SAMFLIX_ENV` at launch — so a preprod build can be replayed locally
//!    without recompiling;
//! 2. `SAMFLIX_ENV` at compile time — set by the CI, based on what it's
//!    building;
//! 3. failing that, `local` — a binary compiled by hand is local, whether
//!    it's debug or release.
//!
//! The default matters more than it looks: it's better for a production
//! incident to pass as local than the other way around. The first is
//! noticed because someone is looking for the incident and can't find it;
//! the second silently pollutes the one environment actually being
//! watched.

/// Deployment environment, in Sentry's sense.
mod resolution;

pub use resolution::{current, origin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Compiled and launched on someone's machine.
    Local,
    /// Built by the CI on a working branch.
    Development,
    /// Built by the CI on the main branch, before publishing.
    Preproduction,
    /// Binary published on a tag.
    Production,
}

impl Environment {
    /// Name sent to Sentry.
    pub fn as_str(self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Development => "development",
            Environment::Preproduction => "preproduction",
            Environment::Production => "production",
        }
    }

    /// Reads an environment name, common aliases included.
    ///
    /// `dev`, `preprod` and `staging` are what one types and what the CIs
    /// use; rejecting them would silently fall back to the default, which is
    /// exactly the mistake this is meant to avoid.
    pub fn parse(text: &str) -> Option<Environment> {
        match text.trim().to_ascii_lowercase().as_str() {
            "local" | "dev-local" => Some(Environment::Local),
            "development" | "dev" => Some(Environment::Development),
            "preproduction" | "preprod" | "pre-prod" | "staging" => {
                Some(Environment::Preproduction)
            }
            "production" | "prod" => Some(Environment::Production),
            _ => None,
        }
    }

    /// Does this environment correspond to a distributed binary?
    ///
    /// Used to decide what's acceptable by default: we're more talkative on
    /// a development machine than for a player.
    pub fn is_deployed(self) -> bool {
        matches!(self, Environment::Preproduction | Environment::Production)
    }
}
