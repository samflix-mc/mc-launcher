//! The facade over `minecraft-auth`.
//!
//! The library's manager is **stateful**: it keeps the tokens for each stage
//! and refreshes them lazily on access. That's the opposite of the old
//! hand-written chain, which started from scratch on every call — and it's
//! exactly what's wanted here: a session resumed from one launch to the
//! next without asking for a code again.

use anyhow::{Context, Result};
use minecraft_auth::java::JavaAuthManager;

mod code;

pub use code::DeviceCode;

pub struct Auth {
    manager: JavaAuthManager,
}

impl Auth {
    /// Opens a session by *device code*.
    ///
    /// `on_code` receives what's needed to guide the user, then the call
    /// blocks until they've authorized — or the code expires.
    pub async fn login(on_code: impl FnOnce(&DeviceCode)) -> Result<Self> {
        let manager = JavaAuthManager::builder(client()?)
            .login_device_code(|code| {
                on_code(&DeviceCode {
                    user_code: code.user_code.clone(),
                    verification_uri: code.verification_uri.clone(),
                    direct_verification_uri: code.direct_verification_uri(),
                });
            })
            .await
            .context("Microsoft sign-in")?;
        Ok(Self { manager })
    }

    /// Resumes a persisted session, without asking for a code again.
    ///
    /// Contacts no one: the tokens are only refreshed on the first access
    /// that needs it.
    pub fn resume(state: &serde_json::Value) -> Result<Self> {
        let manager =
            JavaAuthManager::from_json(client()?, state).context("saved session is unreadable")?;
        Ok(Self { manager })
    }

    /// The full state, to be saved again after each use.
    ///
    /// Since the refresh is lazy, the state after a call isn't the one from
    /// before: not writing it back would leave the next launch starting from
    /// a stale token, and asking for a code again for nothing.
    ///
    /// Out of scope for mutation testing: this needs a `JavaAuthManager` to
    /// call, and `from_json` only builds one from a real state — obtainable
    /// only by opening an actual Microsoft session. The format is internal
    /// to the library; faking it would amount to checking our imitation
    /// rather than its behavior.
    #[mutants::skip]
    pub async fn state(&self) -> Result<serde_json::Value> {
        self.manager
            .to_json()
            .await
            .context("serializing the session")
    }

    /// The player and their token, ready for the game's command line.
    pub async fn session(&self) -> Result<crate::Session> {
        let session = self
            .manager
            .launch_session()
            .await
            .context("opening the game session")?;
        Ok(crate::Session {
            minecraft_token: session.access_token,
            profile: crate::Profile {
                // Without dashes, like `offline_session` and like Prism:
                // it's the form the game expects.
                id: session.player_uuid.simple().to_string(),
                name: session.player_name,
            },
        })
    }

    /// Does the account own the game? An empty inventory counts as "no".
    ///
    /// Out of scope for mutation testing, for the same reason as
    /// [`Auth::state`]: the call goes to Minecraft Services at an address
    /// hard-coded in `minecraft-auth`, which nothing lets us redirect to a
    /// test server. Checking it would require forking the library.
    #[mutants::skip]
    pub async fn owns_game(&self) -> Result<bool> {
        let ents = self
            .manager
            .entitlements()
            .await
            .context("checking the license")?;
        Ok(!ents.items.is_empty())
    }
}

/// The HTTP client for authentication calls.
///
/// `minecraft-auth` sets its own timeout per request; this one only serves
/// to identify the launcher and fix the TLS stack.
fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("samflix-mc-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("building the HTTP client")
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod tests;
