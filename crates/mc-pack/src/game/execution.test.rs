//! What the launcher does with the game's output and its verdict.
//!
//! The JVM for the fixture game sessions is `/bin/sh`: what's being checked
//! isn't Minecraft, but the interpretation of what a process leaves behind.

use super::{GameSession, play};
use crate::fixtures::{Workshop, entry, lock};
use mc_instance::launch::{Command, Outcome, Session};

#[cfg(unix)]
fn game_session(workshop: &Workshop, script: &str) -> GameSession {
    let options = workshop.options();
    let instance = options.layout.instance("samflix");
    instance.create().unwrap();

    GameSession {
        instance,
        lock: lock(vec![entry("jei", "both", None)]),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "0123456789abcdef"),
        target: None,
        explicit_request: false,
        environment: mc_log::Environment::Production,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_game_session_that_ends_cleanly_returns_a_normal_verdict() {
    let workshop = Workshop::new("execution-ok");
    workshop.installed_pack(vec![entry("jei", "both", None)]);

    let report = play(&game_session(&workshop, "echo 'Stopping worker threads'"))
        .await
        .expect("the game was launched successfully");

    assert_eq!(report.outcome, Outcome::Normal);
    assert!(report.errors.is_empty());
}

/// Closing the game from the keyboard is not a crash: reporting it as one
/// would open an incident for every game session ended that way.
#[cfg(unix)]
#[tokio::test]
async fn an_interrupted_game_session_is_distinguished_from_a_failure() {
    let workshop = Workshop::new("execution-interrupted");
    workshop.installed_pack(Vec::new());

    // 130: what a shell returns for a Ctrl+C.
    let report = play(&game_session(&workshop, "exit 130"))
        .await
        .expect("an interruption is not a launch error");

    assert!(matches!(report.outcome, Outcome::Interrupted { .. }));
}

/// Minecraft catches a lot of exceptions and carries on: they count as much
/// as a crash, and they're the ones we'd never see otherwise.
#[cfg(unix)]
#[tokio::test]
async fn errors_caught_along_the_way_are_retained_even_if_the_game_session_ends_cleanly() {
    let workshop = Workshop::new("execution-errors");
    workshop.installed_pack(vec![entry("jei", "both", None)]);

    let report = play(&game_session(
        &workshop,
        "echo 'java.lang.NullPointerException: nothing'; echo 'carrying on'",
    ))
    .await
    .expect("the game session ended cleanly despite the exception");

    assert_eq!(report.outcome, Outcome::Normal);
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
}

/// An error exit **is not** an error from this function: the game did
/// launch. Returning an `Err` would force the window to reconstruct the exit
/// code from a message.
#[cfg(unix)]
#[tokio::test]
async fn an_error_exit_returns_its_code_without_failing() {
    let workshop = Workshop::new("execution-failure");
    workshop.installed_pack(Vec::new());

    let report = play(&game_session(&workshop, "exit 1"))
        .await
        .expect("the launch itself succeeded");

    assert_eq!(report.outcome, Outcome::Failed { code: 1 });
}

#[cfg(unix)]
#[test]
fn the_games_logs_are_named_within_the_instance() {
    // This is where we send someone looking when a game session went wrong:
    // a wrong path would send the player to an empty folder.
    let workshop = Workshop::new("execution-logs");
    workshop.installed_pack(Vec::new());

    let path = super::logs(&game_session(&workshop, "true"));

    assert!(path.ends_with("logs"), "{}", path.display());
    assert!(path.starts_with(&workshop.root), "{}", path.display());
}
