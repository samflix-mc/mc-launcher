//! The binary's four session commands.

use anyhow::{Context, Result};

use mc_auth::{Auth, offline_session};

/// Opens a session and saves it.
///
/// Out of scope for mutation testing: all this command does is talk to
/// Microsoft, through a device code and a wait for human validation. The
/// addresses are `minecraft-auth`'s own, which can't be redirected to a test
/// server. What can be checked — reading the arguments, the dispatch,
/// displaying a profile — is, by the suite that runs the binary; what's left
/// here requires a real account.
#[mutants::skip]
pub async fn login() -> Result<()> {
    let auth = Auth::login(|code| {
        println!("\n  Open {}", code.direct_verification_uri);
        println!(
            "  (or {} and enter {})",
            code.verification_uri, code.user_code
        );
        println!("\n  Waiting for validation…");
    })
    .await?;

    let session = auth.session().await?;
    mc_auth::save(&auth.state().await?)?;

    println!("\nSigned in.");
    display(&session);
    if !auth.owns_game().await? {
        println!("\n  ⚠ this account does not own Minecraft Java Edition.");
    }
    Ok(())
}

/// Shows the saved session, refreshing it if needed.
pub async fn whoami() -> Result<()> {
    let Some(state) = mc_auth::load() else {
        println!("No session — run \"mc-auth login\".");
        return Ok(());
    };

    let auth = Auth::resume(&state)?;
    let session = auth
        .session()
        .await
        .context("the saved session is no longer valid — rerun \"mc-auth login\"")?;

    // The refresh is lazy: whatever the call above renewed would be lost
    // without this rewrite, and the next launch would start from a stale
    // token.
    mc_auth::save(&auth.state().await?)?;

    println!("Session saved to {}", mc_auth::path().display());
    display(&session);
    Ok(())
}

/// Forgets the session.
pub fn logout() -> Result<()> {
    mc_auth::erase()?;
    println!("Session forgotten.");
    Ok(())
}

/// Local profile, without contacting anyone.
pub fn offline(nickname: &str) {
    let session = offline_session(nickname);
    println!("Offline profile — no token, online-mode=false server only.");
    display(&session);
}

fn display(session: &mc_auth::Session) {
    println!("  nickname : {}", session.profile.name);
    println!("  uuid     : {}", session.profile.id);
}

#[cfg(test)]
#[path = "commands.test.rs"]
mod tests;
