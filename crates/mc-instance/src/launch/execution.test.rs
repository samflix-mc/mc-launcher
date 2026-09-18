use super::{Outcome, run};
use crate::launch::command::Command;

/// `/bin/sh` stands in for the JVM: what's checked here isn't Minecraft but
/// the loop that reads its output, writes it back out, and picks exceptions
/// out of it.
#[cfg(unix)]
fn fake_game(script: &str) -> Command {
    Command {
        java: std::path::PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        working_dir: std::env::temp_dir(),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_session_that_ends_cleanly_reports_nothing() {
    let _workshop = crate::fixtures::workshop();
    let report = run(&fake_game("echo 'Stopping worker threads'"))
        .await
        .expect("the process starts");

    assert_eq!(report.outcome, Outcome::Normal);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

/// Minecraft catches a lot of exceptions and carries on: those errors don't
/// show up anywhere else, and they're often the ones that explain a
/// behavior reported much later.
#[cfg(unix)]
#[tokio::test]
async fn an_exception_raised_during_a_session_is_retained() {
    let _workshop = crate::fixtures::workshop();
    let report = run(&fake_game(
        "echo 'java.lang.NullPointerException: nothing at all'; \
         echo '	at net.minecraft.Foo(Foo.java:1)'; \
         echo 'the session continues'",
    ))
    .await
    .unwrap();

    assert_eq!(report.outcome, Outcome::Normal);
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].exception, "java.lang.NullPointerException");
}

/// The two streams are merged: Minecraft writes to both without any
/// meaningful distinction, and an exception on stderr counts just as much
/// as the others.
#[cfg(unix)]
#[tokio::test]
async fn stderr_is_read_just_like_stdout() {
    let _workshop = crate::fixtures::workshop();
    let report = run(&fake_game(
        "echo 'java.io.IOException: disk full' >&2; exit 1",
    ))
    .await
    .unwrap();

    assert_eq!(report.outcome, Outcome::Failed { code: 1 });
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_java_is_reported_with_its_path() {
    let _workshop = crate::fixtures::workshop();
    let command = Command {
        java: std::path::PathBuf::from("/usr/lib/jvm/does-not-exist/bin/java"),
        args: Vec::new(),
        working_dir: std::env::temp_dir(),
    };

    let error = run(&command).await.expect_err("no binary at that location");
    assert!(format!("{error:#}").contains("does-not-exist"), "{error:#}");
}
