//! What `SAMFLIX_NO_KEYRING` decides.
//!
//! The variable exists for two reasons, and the second one cost a Microsoft
//! session on every `cargo test`: the file isolates itself by moving
//! `XDG_CONFIG_HOME`, the keyring doesn't — it's unique to the user's
//! session. A suite that runs `mc-auth logout` would then erase the real
//! session of the machine running it.

use super::keyring_allowed;

/// Serializes the tests that set the variable: two changing it at the same
/// time would contradict each other.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Guard(Option<std::ffi::OsString>);

fn set(value: Option<&str>) -> Guard {
    let previous = std::env::var_os("SAMFLIX_NO_KEYRING");
    // SAFETY: the lock guarantees no other test in this binary reads or
    // writes the variable while the guard lives.
    unsafe {
        match value {
            Some(value) => std::env::set_var("SAMFLIX_NO_KEYRING", value),
            None => std::env::remove_var("SAMFLIX_NO_KEYRING"),
        }
    }
    Guard(previous)
}

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            match self.0.take() {
                Some(value) => std::env::set_var("SAMFLIX_NO_KEYRING", value),
                None => std::env::remove_var("SAMFLIX_NO_KEYRING"),
            }
        }
    }
}

#[test]
fn without_the_variable_the_keyring_is_used() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _guard = set(None);

    assert!(keyring_allowed());
}

#[test]
fn the_three_accepted_forms_cut_off_the_keyring() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    for value in ["1", "true", "yes"] {
        let _guard = set(Some(value));
        assert!(!keyring_allowed(), "\"{value}\" should have cut it off");
    }
}

/// A variable set to anything else cuts off nothing: "0" means "use the
/// keyring", and interpreting it as a presence would cut it off — the
/// opposite of what's being asked.
#[test]
fn a_value_that_does_not_say_yes_leaves_the_keyring_alone() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    for value in ["0", "false", "no", ""] {
        let _guard = set(Some(value));
        assert!(keyring_allowed(), "\"{value}\" should not have cut it off");
    }
}
