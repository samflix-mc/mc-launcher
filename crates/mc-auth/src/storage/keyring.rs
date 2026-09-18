//! The system keyring, when the machine has one.
//!
//! The authentication state carries a refresh token: it reopens the
//! player's account with no password and no second factor. It's the
//! heaviest secret the launcher holds.
//!
//! So it goes to the keyring — Secret Service on Linux, Keychain on macOS,
//! Credential Manager on Windows. Encrypted at rest, unlocked by the user's
//! session, out of reach of a backup that would sweep up `~/.config` or an
//! unlucky `cat`.
//!
//! ## When the machine doesn't have one
//!
//! A container, a session without a wallet, an integration server: there's
//! then nothing to talk to. Refusing to sign in there would be worse than
//! the file, and it's [`super::file`] that takes over — as `0600`, and with
//! a `warn` in the log. It isn't the normal path, and it should be visible
//! as one.

use anyhow::{Context, Result};
use serde_json::Value;

/// The service, as it appears in the keyring.
const SERVICE: &str = "samflix-mc";

/// The entry. There's only one: the launcher only knows one account at a
/// time.
const ENTRY: &str = "session-minecraft";

/// The session saved in the keyring.
///
/// `Ok(None)` means "nothing saved here" — the caller will check the file.
/// An `Err` means the keyring exists but didn't answer, which isn't the
/// same thing and must show up in the log.
/// Out of scope for mutation testing: this function talks to the system
/// keyring. A runner doesn't have one, and on a development machine the
/// test would write to the user's wallet — which is exactly what the
/// `the_keyring_keeps_what_it_is_given` test avoids by being `#[ignore]`
/// and using a service of its own.
///
/// The decision it carries, though, is proven: `absent` tells apart
/// "nothing saved" from "the keyring didn't answer", and two tests hold
/// that line.
#[mutants::skip]
pub(super) fn load() -> Result<Option<Value>> {
    let entry = keyring::Entry::new(SERVICE, ENTRY).context("opening the keyring")?;
    let raw = match entry.get_password() {
        Ok(raw) => raw,
        Err(error) if absent(&error) => return Ok(None),
        Err(error) => return Err(anyhow::Error::new(error).context("reading the keyring")),
    };

    // A secret that's present but unreadable means "no session": the caller
    // will offer to sign in again, where a fatal error would prevent
    // playing because of a corrupted entry. The content isn't logged.
    match serde_json::from_str(&raw) {
        Ok(state) => Ok(Some(state)),
        Err(error) => {
            tracing::warn!(error = %error, "keyring session unreadable, sign-in required again");
            Ok(None)
        }
    }
}

pub(super) fn save(state: &Value) -> Result<()> {
    let raw = serde_json::to_string(state).context("serializing the session")?;
    let entry = keyring::Entry::new(SERVICE, ENTRY).context("opening the keyring")?;
    entry.set_password(&raw).context("writing to the keyring")
}

/// Out of scope for mutation testing: this function talks to the system
/// keyring. A runner doesn't have one, and on a development machine the
/// test would write to the user's wallet — which is exactly what the
/// `the_keyring_keeps_what_it_is_given` test avoids by being `#[ignore]`
/// and using a service of its own.
///
/// The decision it carries, though, is proven: `absent` tells apart
/// "nothing saved" from "the keyring didn't answer", and two tests hold
/// that line.
#[mutants::skip]
pub(super) fn erase() -> Result<()> {
    let entry = match keyring::Entry::new(SERVICE, ENTRY) {
        Ok(entry) => entry,
        // No keyring at all: there's nothing to forget there, and the file
        // is still to be erased. This isn't an error.
        Err(error) => {
            tracing::warn!(error = %error, "keyring unavailable, nothing to erase there");
            return Ok(());
        }
    };
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(error) if absent(&error) => Ok(()),
        Err(error) => Err(anyhow::Error::new(error).context("erasing from the keyring")),
    }
}

/// The entry doesn't exist — which isn't a failure.
///
/// The distinction carries the whole module: "nothing saved" leads to the
/// file, "the keyring is locked" must show up in the log.
fn absent(error: &keyring::Error) -> bool {
    matches!(error, keyring::Error::NoEntry)
}

#[cfg(test)]
#[path = "keyring.test.rs"]
mod tests;
