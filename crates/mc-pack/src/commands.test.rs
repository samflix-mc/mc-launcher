use super::execute;
use super::fixtures::{Workshop, entry};
use std::process::ExitCode;

async fn run(command: &str, workshop: &Workshop) -> ExitCode {
    let source = workshop.installed_pack(vec![entry("jei", "both")]);
    execute(
        command,
        &source,
        &workshop.options(),
        false,
        None,
        None,
        None,
        false,
    )
    .await
    .expect("the command finishes")
}

/// `verify` is the only one that distinguishes two successes: the
/// installation matches the lock, or it doesn't without the program having
/// failed for that.
#[tokio::test]
async fn verify_distinguishes_two_successes() {
    let workshop = Workshop::new("execute-verify");
    let code = run("verify", &workshop).await;
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

/// An unknown command recalls usage and returns 2 — the code shells expect
/// from a malformed command line.
#[tokio::test]
async fn an_unknown_command_recalls_usage() {
    let workshop = Workshop::new("execute-unknown");
    let code = run("instal", &workshop).await;
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
}
