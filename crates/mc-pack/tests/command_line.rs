//! The binary as a player invokes it.
//!
//! mc-pack is the command whose existence justifies this repo: it's the one
//! a player launches. Its entry point, its dispatch, and its help page
//! aren't reachable from the crate — and yet they're what decides what a
//! player sees when they get it wrong.
//!
//! Only commands that download nothing are exercised here: a suite that
//! installed a thousand-mod pack would hit Modrinth on every run.

use std::path::Path;
use std::process::{Command, Output};

fn mc_pack(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-pack"))
        .args(args)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("HOME", home)
        // Nothing should reach Sentry because a suite ran.
        .env("SAMFLIX_TELEMETRY", "0")
        // No suite touches the machine's keyring: unlike the session file,
        // it isn't isolated by a path variable.
        .env("SAMFLIX_NO_KEYRING", "1")
        .output()
        .expect("the mc-pack binary was built by cargo test")
}

fn workshop(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mc-pack-cli-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Without an argument, the help page goes to standard error and the code
/// is 2.
///
/// Two, not one: that's the convention distinguishing "I didn't understand
/// your command" from "your command failed". A script that retries on
/// failure must not retry a broken command line.
#[test]
fn without_an_argument_the_help_is_shown_and_the_code_is_two() {
    let home = workshop("usage");
    let output = mc_pack(&[], &home);

    assert_eq!(output.status.code(), Some(2), "unexpected code");

    let help = String::from_utf8_lossy(&output.stderr);
    for command in ["install", "lock", "verify", "launch", "diagnostic"] {
        assert!(
            help.contains(command),
            "\"{command}\" missing from the help"
        );
    }
    // The help names the pack that gets installed if nothing is said: it's
    // the only way to know, from a machine, which environment this binary
    // serves.
    assert!(help.contains("Default"), "{help}");

    std::fs::remove_dir_all(&home).ok();
}

/// "diagnostic" is the first thing to ask someone whose installation is
/// failing: it says where the logs are and whether incidents get reported.
/// It reads no manifest and touches nothing — which is exactly what you run
/// when you don't yet know what's wrong.
#[test]
fn diagnostic_says_where_the_logs_are_and_where_incidents_go() {
    let home = workshop("diagnostic");
    let output = mc_pack(&["diagnostic"], &home);

    assert!(
        output.status.success(),
        "errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Logs"), "{text}");
    assert!(text.contains("Incident reporting"), "{text}");
    // The announced directory is indeed the one it was given: a diagnostic
    // that names the wrong path sends you looking for the log where it
    // isn't.
    assert!(
        text.contains(home.join("data").to_str().unwrap()),
        "path outside the declared directory: {text}"
    );
    // Telemetry is disabled by this test's environment: the diagnostic must
    // say so, or it would announce a report that won't happen.
    assert!(text.contains("disabled"), "{text}");

    std::fs::remove_dir_all(&home).ok();
}

/// An unknown command isn't an empty command: it names itself, so the typo
/// jumps out.
#[test]
fn an_unknown_command_is_named() {
    let home = workshop("unknown");
    let output = mc_pack(&["instal"], &home);

    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("instal"), "{errors}");

    std::fs::remove_dir_all(&home).ok();
}
