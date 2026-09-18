//! Minecraft Java authentication: online, or offline.
//!
//! ## Why this crate no longer talks to an Azure application
//!
//! The launcher long presented its own Azure registration. On 2026-09-16,
//! Mojang Enforcement refused it for the Minecraft API allowlist, with no
//! reason given and no appeal. The Microsoft → Xbox Live → XSTS chain still
//! worked: only `api.minecraftservices.com` answered 403,
//! `{"errorMessage":"Invalid app registration"}`, purely on the strength of
//! the application ID.
//!
//! The path taken is the one LiquidBounce took, having hit the same refusal:
//! authenticate with the identity of the **official launcher**
//! (`00000000402b5328`, a *title ID*, not an Azure UUID), through the
//! `minecraft-auth` crate. The flow isn't the same — legacy MSA, an
//! ECDSA P-256-signed device token, SISU in a single call, `/launcher/login`
//! — and it's the library that carries it.
//!
//! ## What this choice costs
//!
//! This isn't an approval obtained, it's a filter bypassed. It breaches
//! Microsoft's and Mojang's terms of service. No ban tied to this method is
//! documented so far, but the residual risk would fall on **the players**,
//! not just the maintainer. The README states it, so the choice is informed.
//!
//! ## Offline
//!
//! [`offline_session`] stays, unchanged: it serves development and
//! `online-mode=false` servers, including the network's own. It depends on
//! none of the above.

mod auth;
mod offline;
mod storage;

pub use auth::{Auth, DeviceCode};
pub use offline::offline_session;
pub use storage::{erase, load, path, save};

/// The player, as the game must announce them.
///
/// `id` is the UUID in hexadecimal **without dashes**: it's the form the game
/// expects on its command line, and the one [`offline_session`] already
/// produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

/// A session ready to launch the game.
///
/// The token is empty in offline mode: that's the only visible difference
/// from here, and the game copes with it as long as the server runs
/// `online-mode=false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub minecraft_token: String,
    pub profile: Profile,
}

impl Session {
    /// Does this session open an online server?
    pub fn is_online(&self) -> bool {
        !self.minecraft_token.is_empty()
    }
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
