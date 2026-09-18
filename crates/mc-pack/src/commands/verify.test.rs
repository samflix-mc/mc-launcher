use super::{report_unresolved, verify};
use crate::commands::fixtures::{Workshop, entry, lockfile};
use std::process::ExitCode;

/// A matching installation returns success; the caller uses it as the exit
/// code, and it's what carries the information in CI.
#[test]
fn a_matching_installation_returns_success() {
    let workshop = Workshop::new("verify-ok");
    let source = workshop.installed_pack(vec![entry("jei", "both")]);

    let code = verify(&source, &workshop.options(), false).unwrap();
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

/// A verification mismatch isn't a program failure: it describes the
/// installation, and it's the exit code that carries it.
#[test]
fn an_incomplete_installation_returns_failure_without_panicking() {
    let workshop = Workshop::new("verify-ko");
    let source = workshop.installed_pack(vec![entry("jei", "both")]);
    let instance = workshop.options().layout.instance("samflix");
    std::fs::remove_file(instance.mods_dir().join("jei.jar")).unwrap();

    let code = verify(&source, &workshop.options(), false).unwrap();
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
}

/// A missing dependency doesn't stop the install but it will stop the game
/// from starting: it's flagged where it'll be seen.
#[test]
fn missing_dependencies_are_announced() {
    let mut lock = lockfile(vec![entry("jei", "both")]);
    lock.unresolved.push(mc_pack::lockfile::LockedMissing {
        mod_id: "bookshelf".into(),
        required_by: "jei".into(),
        side: "both".into(),
    });

    // It goes to standard error; what the test checks is that the list is
    // walked without panicking, including when it's empty.
    report_unresolved(&lock);
    report_unresolved(&lockfile(Vec::new()));
}
