//! What can be checked without a keyring.
//!
//! Writing to the Secret Service from a test suite would require a session
//! bus and an unlocked wallet — that is, testing the environment rather
//! than the code. What's left, and what decides everything else in this
//! module, is the dispatch: which error means "nothing saved" and which
//! error is a failure.

use super::{ENTRY, SERVICE, absent};
use keyring::Error;

#[test]
fn absence_is_recognized() {
    assert!(absent(&Error::NoEntry));
}

#[test]
fn a_failure_is_not_confused_with_an_absence() {
    // A locked keyring, a platform with no store, an ambiguous entry: none
    // of them mean the player isn't signed in, and treating them as such
    // would silently fall back to the file.
    assert!(!absent(&Error::NoDefaultStore));
    assert!(!absent(&Error::BadEncoding(vec![0xff])));
    assert!(!absent(&Error::Invalid(
        "service".to_string(),
        "empty".to_string()
    )));
}

#[test]
fn the_service_and_the_entry_do_not_move() {
    // Renaming either one makes the already-saved session invisible: the
    // player ends up signed out with no explanation, and the old entry
    // stays in their keyring with a valid token inside.
    assert_eq!(SERVICE, "samflix-mc");
    assert_eq!(ENTRY, "session-minecraft");
}

/// Does this machine's keyring really keep what it's given?
///
/// Ignored by default: it writes to the user's wallet, which a suite must
/// not do unless asked. Run it when a session evaporates from one launch to
/// the next:
///
/// ```sh
/// cargo test -p mc-auth -- --ignored --nocapture the_keyring_keeps
/// ```
///
/// Writes under a service distinct from the launcher's, so as not to
/// overwrite the current session, and cleans up after itself.
#[test]
#[ignore = "touches the machine's wallet"]
fn the_keyring_keeps_what_it_is_given() {
    const TEST: &str = "samflix-mc-test";

    let entry = keyring::Entry::new(TEST, "diagnostic").expect("the keyring opens");
    entry.set_password("witness-value").expect("write");

    // Read back through a fresh entry: the first one may well have kept the
    // value in memory without anything being persisted.
    let reread = keyring::Entry::new(TEST, "diagnostic")
        .expect("the keyring opens")
        .get_password();

    // `MC_KEYRING_WITNESS=1` leaves the entry in place: that's what lets it
    // be checked from another process — `secret-tool lookup service
    // samflix-mc-test username diagnostic` — that it was really persisted
    // and not just kept in memory by the wallet.
    if std::env::var_os("MC_KEYRING_WITNESS").is_none() {
        let _ = entry.delete_credential();
    }

    assert_eq!(
        reread.as_deref().ok(),
        Some("witness-value"),
        "the keyring accepted the write without returning it: {reread:?}"
    );
}
