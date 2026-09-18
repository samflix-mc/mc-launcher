use super::{run_installer, tail};

#[test]
fn the_tail_keeps_the_last_lines() {
    assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail("a", 5), "a");
    assert_eq!(tail("", 5), "");
}

/// `/bin/echo` stands in for a JVM: what's verified isn't the NeoForge
/// installer but the way it's called and its verdict read.
#[cfg(unix)]
#[tokio::test]
async fn a_successful_installer_says_nothing() {
    let _workshop = crate::fixtures::workshop();
    let dir = std::env::temp_dir();
    run_installer(
        std::path::Path::new("installer.jar"),
        "--install-client",
        &dir,
        std::path::Path::new("/bin/echo"),
    )
    .await
    .expect("zero exit code");
}

/// The installer writes its useful diagnostic to stdout and traces to
/// stderr; both are needed to understand a failure, and a player won't go
/// looking for them on their own.
#[cfg(unix)]
#[tokio::test]
async fn a_failure_reports_both_outputs() {
    let _workshop = crate::fixtures::workshop();
    let script = std::env::temp_dir().join(format!("mc-neoforge-failure-{}", std::process::id()));
    std::fs::write(
        &script,
        "#!/bin/sh\necho 'Failed to install: wrong version'\necho 'java.io.IOException' >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0o700: only the process that just wrote this script runs it, and it
        // lives in a temp directory shared by the whole machine.
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    let error = run_installer(
        std::path::Path::new("installer.jar"),
        "--install-client",
        &std::env::temp_dir(),
        &script,
    )
    .await
    .expect_err("non-zero exit code");
    std::fs::remove_file(&script).ok();

    let text = format!("{error:#}");
    assert!(text.contains("Failed to install"), "{text}");
    assert!(text.contains("java.io.IOException"), "{text}");
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_java_names_the_installer() {
    let _workshop = crate::fixtures::workshop();
    let error = run_installer(
        std::path::Path::new("/cache/neoforge-21.1.250-installer.jar"),
        "--install-client",
        &std::env::temp_dir(),
        std::path::Path::new("/usr/lib/jvm/absent/bin/java"),
    )
    .await
    .expect_err("no binary at this location");

    assert!(
        format!("{error:#}").contains("neoforge-21.1.250-installer.jar"),
        "{error:#}"
    );
}
