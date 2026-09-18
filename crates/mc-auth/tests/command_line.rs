//! The binary as a player invokes it.
//!
//! Everything above is checked function by function, in the crate. What's
//! left is what no direct call reaches: the entry point, the dispatch to
//! the command, the exit code returned to the shell, and what gets printed.
//! These four are only visible by running the executable — a function that
//! no longer prints anything doesn't fail any test that merely calls it.

use std::path::Path;
use std::process::{Command, Output};

/// Runs the binary in a directory of its own.
///
/// `mc_log::init` opens a log in the data directory from the first line of
/// `main`: without these variables, every run would write into the one used
/// by the machine running the suite.
fn mc_auth(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-auth"))
        .args(args)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("HOME", home)
        // Nothing should reach Sentry because a suite ran.
        .env("SAMFLIX_TELEMETRY", "0")
        // And above all: no keyring. The session file isolates itself by
        // moving XDG_CONFIG_HOME, the keyring doesn't — it's unique to the
        // user's session. Without this variable, "logout" would erase the
        // real Microsoft session of the machine running the suite, and the
        // player would have to sign in again after every "cargo test".
        .env("SAMFLIX_NO_KEYRING", "1")
        .output()
        .expect("the mc-auth binary was built by cargo test")
}

fn workshop(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mc-auth-cli-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// The offline profile is the only command that succeeds without Microsoft:
/// it's therefore the only one that checks, end to end, that a nickname
/// goes in one side and an identity comes out the other.
#[test]
fn the_offline_profile_shows_the_nickname_and_its_uuid() {
    let home = workshop("offline");
    let output = mc_auth(&["--offline", "Notch"], &home);

    assert!(
        output.status.success(),
        "code {:?}, errors: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Notch"), "{text}");
    // The UUID an online-mode=false server computes for this nickname.
    // Seeing it here proves the display renders what the computation
    // produced.
    assert!(text.contains("b50ad385829d3141a2167e7d7539ba7f"), "{text}");
    // And that it says what this session doesn't allow.
    assert!(text.contains("online-mode=false"), "{text}");

    std::fs::remove_dir_all(&home).ok();
}

/// Without a command, the binary must recall what it expects *and* exit
/// with an error. A zero exit code would make a calling script believe the
/// session is open.
#[test]
fn without_a_command_the_usage_is_recalled_and_the_code_is_nonzero() {
    let home = workshop("no-command");
    let output = mc_auth(&[], &home);

    assert!(
        !output.status.success(),
        "an empty invocation cannot succeed"
    );

    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("mc-auth login"), "{errors}");
    assert!(errors.contains("--offline <NICKNAME>"), "{errors}");

    std::fs::remove_dir_all(&home).ok();
}

/// `logout` without a session isn't an error: it's the state we wanted to
/// reach. The exit code must say so.
#[test]
fn forgetting_an_absent_session_succeeds() {
    let home = workshop("logout");
    let output = mc_auth(&["logout"], &home);

    assert!(
        output.status.success(),
        "errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("forgotten"), "{text}");

    std::fs::remove_dir_all(&home).ok();
}

/// `whoami` without a saved session points to `login` rather than failing:
/// it's the case of a first launch, not an outage.
#[test]
fn naming_yourself_without_a_session_points_to_login() {
    let home = workshop("whoami");
    let output = mc_auth(&["whoami"], &home);

    assert!(
        output.status.success(),
        "errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("mc-auth login"), "{text}");

    std::fs::remove_dir_all(&home).ok();
}
