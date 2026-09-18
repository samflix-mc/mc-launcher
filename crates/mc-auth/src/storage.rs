//! Where the session lives between two launches.
//!
//! The content is the full state of `minecraft-auth`: it carries a refresh
//! token, which reopens the player's account with no password and no second
//! factor. It's the heaviest secret the launcher holds.
//!
//! Two places, in this order:
//!
//! 1. **the system keyring** — Secret Service, Keychain, Credential
//!    Manager. Encrypted at rest, unlocked by the user's session, out of
//!    reach of a backup that would sweep up `~/.config`;
//! 2. **a `0600` file** — `~/.config/samflix-mc/session.json`, for machines
//!    with no keyring: a container, a session without a wallet, an
//!    integration runner. The fallback is logged at `warn`; it isn't the
//!    normal path, and it should be visible as one.
//!
//! ## Why both, and not either
//!
//! Reading tries the keyring **then** the file. A session opened from the
//! terminal before the keyring becomes available stays usable this way, and
//! the reverse too — the window and the command line share the account
//! without the player having to sign in twice.
//!
//! Writing, meanwhile, doesn't touch the file as long as the keyring
//! answers. It doesn't erase it either: deleting out from under a session
//! another command just opened would break it. Only [`erase`] — signing
//! out, which is requested — clears both.
//!
//! ## Opting out
//!
//! `SAMFLIX_NO_KEYRING=1` takes the keyring out of the loop: everything goes
//! through the `0600` file. Two uses, and the second isn't the less
//! important one.
//!
//! For a machine where the wallet is more of a nuisance than a service — a
//! GPG-encrypted kdewallet that asks for its passphrase on every access —
//! it's a way out.
//!
//! For test suites, it's a necessity. The file isolates itself by moving
//! `XDG_CONFIG_HOME`; the keyring doesn't: it's unique to the user's
//! session. A test that runs `mc-auth logout` would then erase the real
//! session of the machine running the suite — which happened, and showed up
//! as a required Microsoft sign-in after every `cargo test`.

mod file;
mod keyring;

use anyhow::Result;

pub use file::path;

/// Is the keyring usable?
///
/// Read on every call rather than cached: a suite sets the variable for the
/// subprocess it spawns, and nothing guarantees the order in which the first
/// accesses happen.
fn keyring_allowed() -> bool {
    !matches!(
        std::env::var("SAMFLIX_NO_KEYRING").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// The saved session, or `None` if no one has signed in here.
/// Out of scope for mutation testing, and the reason is stronger than a
/// difficulty: flipping the `!keyring_allowed()` guard would make the suite
/// talk to the REAL keyring of the machine running it — reading, writing or
/// erasing the user's wallet. A test must not be able to do that, and a
/// mutant even less so.
///
/// What can safely be checked, is: `keyring_allowed` is proven on its own
/// for the four forms the variable accepts, and the whole file half —
/// path, `0600` mode, round trip — has its own tests. What's left here is
/// the dispatch between the two, and testing it would require injecting the
/// keyring behind a trait to be mutated without side effects.
#[mutants::skip]
pub fn load() -> Option<serde_json::Value> {
    if !keyring_allowed() {
        return file::load_from(&path());
    }
    match keyring::load() {
        Ok(Some(state)) => Some(state),
        // Nothing in the keyring: the file is all that's left.
        Ok(None) => file::load_from(&path()),
        Err(error) => {
            tracing::warn!(error = %error, "keyring unreadable, reading the session file");
            file::load_from(&path())
        }
    }
}

/// Writes the session, to the keyring if the machine has one.
/// Out of scope for mutation testing, and the reason is stronger than a
/// difficulty: flipping the `!keyring_allowed()` guard would make the suite
/// talk to the REAL keyring of the machine running it — reading, writing or
/// erasing the user's wallet. A test must not be able to do that, and a
/// mutant even less so.
///
/// What can safely be checked, is: `keyring_allowed` is proven on its own
/// for the four forms the variable accepts, and the whole file half —
/// path, `0600` mode, round trip — has its own tests. What's left here is
/// the dispatch between the two, and testing it would require injecting the
/// keyring behind a trait to be mutated without side effects.
#[mutants::skip]
pub fn save(state: &serde_json::Value) -> Result<()> {
    if !keyring_allowed() {
        return file::save_to(&path(), state);
    }
    match keyring::save(state) {
        Ok(()) => {
            tracing::debug!("session saved to the system keyring");
            Ok(())
        }
        Err(error) => {
            tracing::warn!(error = %error, "keyring unavailable, falling back to the 0600 file");
            file::save_to(&path(), state)
        }
    }
}

/// Forgets the session, on both sides.
///
/// Both erasures are attempted before returning: failing on one would leave
/// the other in place, and a token believed deleted is worse than a token
/// known to be present.
/// Out of scope for mutation testing, and the reason is stronger than a
/// difficulty: flipping the `!keyring_allowed()` guard would make the suite
/// talk to the REAL keyring of the machine running it — reading, writing or
/// erasing the user's wallet. A test must not be able to do that, and a
/// mutant even less so.
///
/// What can safely be checked, is: `keyring_allowed` is proven on its own
/// for the four forms the variable accepts, and the whole file half —
/// path, `0600` mode, round trip — has its own tests. What's left here is
/// the dispatch between the two, and testing it would require injecting the
/// keyring behind a trait to be mutated without side effects.
#[mutants::skip]
pub fn erase() -> Result<()> {
    if !keyring_allowed() {
        return file::erase_from(&path());
    }
    let keyring = keyring::erase();
    let file = file::erase_from(&path());
    keyring.and(file)
}

#[cfg(test)]
#[path = "storage.test.rs"]
mod tests;
