use super::{logout, offline, whoami};

/// A config directory of its own for this test, and the variable that points
/// to it.
///
/// SAFETY: the guard sets and unsets `XDG_CONFIG_HOME`; this binary spawns no
/// subprocess, and the tests that depend on it are serialized by the lock
/// below.
struct Configuration {
    root: std::path::PathBuf,
    previous: Option<std::ffi::OsString>,
    previous_keyring: Option<std::ffi::OsString>,
}

static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn configuration(name: &str) -> Configuration {
    use std::sync::atomic::Ordering;
    while BUSY
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }

    let root = std::env::temp_dir().join(format!("mc-auth-cmd-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();

    let previous = std::env::var_os("XDG_CONFIG_HOME");
    let previous_keyring = std::env::var_os("SAMFLIX_NO_KEYRING");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &root);
        // **`XDG_CONFIG_HOME` isolates the FILE, and nothing else.**
        //
        // The keyring has no such lever: it belongs to the user's session,
        // and there is exactly one of it. Without this variable, the
        // `logout` test below calls `erase`, which finds the keyring
        // allowed and wipes the REAL `samflix-mc/session-minecraft` entry
        // of whoever runs the suite.
        //
        // That is not a theory. The symptom is a Microsoft sign-in demanded
        // after every `cargo test`, and it was diagnosed by planting a
        // known value in the entry, running the suite, and finding it gone.
        //
        // `tests/command_line.rs` already sets it for the subprocesses it
        // spawns. These tests call `logout()` IN PROCESS, so they need it
        // here too — and that is precisely what had been missed.
        std::env::set_var("SAMFLIX_NO_KEYRING", "1");
    }
    Configuration {
        root,
        previous,
        previous_keyring,
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        unsafe {
            match self.previous.take() {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match self.previous_keyring.take() {
                Some(value) => std::env::set_var("SAMFLIX_NO_KEYRING", value),
                None => std::env::remove_var("SAMFLIX_NO_KEYRING"),
            }
        }
        std::fs::remove_dir_all(&self.root).ok();
        BUSY.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// A local profile contacts no one: that's what allows playing on an
/// `online-mode=false` server without a Microsoft account.
#[test]
fn the_offline_profile_contacts_no_one() {
    offline("Sam");
    offline("thesam1798");
}

/// Signing out without a saved session isn't an error: it's the state we
/// want to end up in.
#[test]
fn signing_out_without_a_session_stays_a_no_op() {
    let config = configuration("logout");

    logout().expect("no session to forget");
    mc_auth::save(&serde_json::json!({"refresh_token": "M.R3_BAY.x"})).unwrap();
    assert!(mc_auth::load().is_some());

    logout().expect("the session is forgotten");
    assert!(mc_auth::load().is_none());

    drop(config);
}

/// Without a session, `whoami` says so and returns: it's a question, not an
/// operation that can fail.
#[tokio::test]
async fn whoami_without_a_session_does_not_raise_an_error() {
    let config = configuration("whoami");

    whoami().await.expect("no session is not an error");

    drop(config);
}
