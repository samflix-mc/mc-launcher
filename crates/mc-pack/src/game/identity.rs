//! Under what identity the game is launched.
//!
//! The choice is explicit, never guessed: `--nickname` requests an offline
//! session, its absence requests the registered Microsoft account. A silent
//! fallback from one to the other would let a player onto a server under an
//! identity they didn't choose — and, on an online server, a connection
//! refusal with no readable cause.

use anyhow::{Result, bail};

use mc_instance::launch::Session;

/// Under what identity to play.
///
/// An `enum` rather than an `Option<String>`: the latter reads as "maybe a
/// nickname", and the caller has to guess what `None` means. Here both
/// intents are named, and neither is the other's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// The registered account. Required for online servers.
    Microsoft,
    /// A local profile, without a token. `online-mode=false` servers only.
    Offline(String),
}

pub(crate) async fn choose(identity: Identity) -> Result<Session> {
    match identity {
        Identity::Offline(nickname) => Ok(offline(&nickname)),
        Identity::Microsoft => online().await,
    }
}

/// The UUID follows the vanilla server rule, so the player keeps the same one
/// from one game session to the next — inventory, position, and permissions
/// included.
fn offline(nickname: &str) -> Session {
    let profile = mc_auth::offline_session(nickname).profile;
    tracing::info!(nickname = %profile.name, "offline session");
    Session::offline(&profile.name, &profile.id)
}

/// What's said when no one is signed in.
///
/// A constant rather than a literal at the call site: the message must give
/// **both** ways out — sign in, or play offline — and that's the only thing
/// worth checking about it. Testing it by calling [`online`] would mean
/// guaranteeing no session exists on the machine running the suite. The
/// system keyring doesn't move with an environment variable, unlike the file:
/// the test would pass or not depending on whether the developer is signed
/// in, which teaches nothing about the code.
pub(crate) const NO_SESSION: &str = "no session registered.\n\
     Sign in with \"mc-auth login\", or play offline with \"--nickname <NAME>\".";

/// The session registered by `mc-auth login`, refreshed if needed.
async fn online() -> Result<Session> {
    let Some(state) = mc_auth::load() else {
        bail!(NO_SESSION);
    };

    let auth = mc_auth::Auth::resume(&state)?;
    let session = auth.session().await.map_err(|e| {
        e.context("the registered session is no longer valid — rerun \"mc-auth login\"")
    })?;

    // The refresh is lazy: without this rewrite, the next launch would start
    // over from the expired token and ask for a code for nothing.
    mc_auth::save(&auth.state().await?)?;

    tracing::info!(nickname = %session.profile.name, "Microsoft session");
    Ok(Session::online(
        &session.profile.name,
        &session.profile.id,
        &session.minecraft_token,
    ))
}

#[cfg(test)]
#[path = "identity.test.rs"]
mod tests;
