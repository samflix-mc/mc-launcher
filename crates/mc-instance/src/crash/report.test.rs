use super::{find, now};
use crate::fixtures::Tree;

const TRACE: &str = "\
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package com.mojang.blaze3d.systems
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source)";

/// Turns back a file's last-write date, to pass it off as leftover from a
/// previous session.
fn age(path: &std::path::Path, seconds: u64) {
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds);
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(when))
        .unwrap();
}

/// The launch instant, turned back by one second.
///
/// A file's write date is coarser than the clock: a file created right
/// after `now()` can carry a date rounded below it, and pass for a leftover
/// from the previous session. The game, though, runs for minutes between
/// the two — the margin only exists for this test.
fn launch() -> std::time::SystemTime {
    now() - std::time::Duration::from_secs(1)
}

fn write(tree: &Tree, relative: &str, content: &str) -> std::path::PathBuf {
    let path = tree.game_dir().join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

/// The crash report is the richest source: description, trace, loaded mods,
/// graphics driver. It's preferred whenever it exists.
#[test]
fn the_crash_report_is_preferred_over_the_log() {
    let tree = Tree::new("report-preferred");
    let start = launch();
    write(
        &tree,
        "logs/latest.log",
        "java.io.IOException: something else",
    );
    let report = write(&tree, "crash-reports/crash-2026-09-17.txt", TRACE);

    let found = find(&tree.game_dir(), start).expect("a crash is found");

    assert_eq!(found.exception, "java.lang.module.ResolutionException");
    assert_eq!(found.source, report);
}

/// A mod loading error shows up in the log while no report is produced: the
/// JVM stops before the game exists. This is the most common case with a
/// modpack.
#[test]
fn without_a_report_the_log_is_read() {
    let tree = Tree::new("report-log");
    let start = launch();
    let log = write(&tree, "logs/latest.log", TRACE);

    let found = find(&tree.game_dir(), start).expect("the log is enough");

    assert_eq!(found.exception, "java.lang.module.ResolutionException");
    assert_eq!(found.source, log);
}

/// A three-day-old report attributed to today's launch would send things
/// down a false trail.
#[test]
fn a_report_older_than_the_launch_is_ignored() {
    let tree = Tree::new("report-old");
    let path = write(&tree, "crash-reports/crash-old.txt", TRACE);
    age(&path, 3 * 24 * 3600);
    let log = write(&tree, "logs/latest.log", TRACE);
    age(&log, 3 * 24 * 3600);

    assert!(find(&tree.game_dir(), now()).is_none());
}

/// Between two reports from this run, the last one written describes the
/// stop.
#[test]
fn the_most_recent_report_wins() {
    let tree = Tree::new("report-recent");
    let start = launch();
    let old = write(
        &tree,
        "crash-reports/crash-a.txt",
        "java.io.IOException: the first one",
    );
    age(&old, 1);
    let recent = write(&tree, "crash-reports/crash-b.txt", TRACE);

    let found = find(&tree.game_dir(), start).unwrap();
    assert_eq!(found.source, recent);
}

/// A session that ends normally leaves a log with no exception: finding
/// nothing there is the nominal case, not a failure.
#[test]
fn a_session_without_an_incident_gives_nothing() {
    let tree = Tree::new("report-clean");
    let start = launch();
    write(&tree, "logs/latest.log", "[INFO]: Stopping worker threads");

    assert!(find(&tree.game_dir(), start).is_none());
}

#[test]
fn an_empty_game_dir_produces_nothing() {
    let tree = Tree::new("report-empty");
    assert!(find(&tree.game_dir(), now()).is_none());
}

/// Files in the directory that aren't reports — a note, a screenshot —
/// shouldn't be mistaken for one.
#[test]
fn only_files_named_crash_are_examined() {
    let tree = Tree::new("report-foreign");
    let start = launch();
    write(&tree, "crash-reports/notes.txt", TRACE);

    assert!(find(&tree.game_dir(), start).is_none());
}
