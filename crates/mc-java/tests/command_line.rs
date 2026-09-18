//! The binary as an install script would call it.
//!
//! `execute` is verified inside the crate; what isn't is what the program
//! returns to the shell. Yet that's its whole interface: CI and launch
//! scripts read nothing from it but the exit code, and an error that came
//! back as zero would let a script carry on with no Java present.

use std::path::Path;
use std::process::{Command, Output};

fn mc_java(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-java"))
        .args(args)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("HOME", home)
        .env("SAMFLIX_TELEMETRY", "0")
        .output()
        .expect("the mc-java binary was built by cargo test")
}

fn workshop(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mc-java-cli-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// `--check` touches nothing and only says whether this machine has the
/// runtime. A major version nobody publishes is never there: the exit code
/// must say so, or a CI would chain into a launch doomed to fail.
#[test]
fn a_missing_runtime_is_reported_by_the_exit_code() {
    let home = workshop("check-missing");
    let output = mc_java(
        &[
            "--check",
            "--major",
            "999",
            "--dir",
            home.join("runtimes").to_str().unwrap(),
        ],
        &home,
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "output: {} / {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(&home).ok();
}

/// An unknown option stops before installing anything, and says so. A
/// two-hundred-megabyte download triggered by a typo would be a costly
/// surprise.
#[test]
fn an_unknown_option_stops_everything_and_names_itself() {
    let home = workshop("unknown-option");
    let output = mc_java(&["--bogus", "21"], &home);

    assert!(!output.status.success(), "an unknown option cannot succeed");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("--bogus"), "{errors}");

    std::fs::remove_dir_all(&home).ok();
}

/// `--major` expects an integer. The refusal must name what was received:
/// it's the only way to spot an empty shell variable passed without quotes.
#[test]
fn an_unreadable_major_names_what_was_received() {
    let home = workshop("unreadable-major");
    let output = mc_java(&["--major", "twenty-one"], &home);

    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("twenty-one"), "{errors}");

    std::fs::remove_dir_all(&home).ok();
}
