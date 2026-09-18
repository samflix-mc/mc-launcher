//! What the command line writes, and nothing else.
//!
//! Preparation and execution are tested in the library, where they live.
//! What's left here is the display — the summary before launching, and the
//! report of errors caught.

use super::super::fixtures::{Workshop, entry, lockfile};
use super::announce::announce;
use mc_instance::launch::{Command, Session};
use mc_pack::GameSession;

#[cfg(unix)]
fn game_session(workshop: &Workshop, target: Option<&str>, explicit: bool) -> GameSession {
    let options = workshop.options();
    let instance = options.layout.instance("samflix");
    instance.create().unwrap();

    GameSession {
        instance,
        lock: lockfile(vec![entry("jei", "both")]),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), "true".into()],
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "0123456789abcdef"),
        target: target.map(str::to_string),
        explicit_request: explicit,
        environment: mc_log::Environment::Production,
    }
}

/// Where the server comes from is stated, not just its address: someone
/// diagnosing a kick needs to know whether they're looking at the pack or a
/// `--server`.
#[cfg(unix)]
#[test]
fn the_announcement_covers_its_three_origins() {
    let workshop = Workshop::new("announce");
    workshop.installed_pack(Vec::new());

    announce(&game_session(&workshop, Some("mc.ggy.info"), false));
    announce(&game_session(&workshop, Some("test.invalid"), true));
    announce(&game_session(&workshop, None, false));
}

/// Minecraft catches a lot of exceptions and keeps going: these errors show
/// up nowhere else, and they're often what explains a behavior reported
/// much later. Announcing "0 errors" after an incident-free session would
/// send someone looking for a nonexistent problem.
#[test]
fn caught_errors_are_only_announced_when_there_are_any() {
    assert!(super::error_lines(&[]).is_empty());

    let crash = mc_instance::crash::parse("java.lang.NullPointerException: nothing at all")
        .expect("an exception");

    let rendered = super::error_lines(std::slice::from_ref(&crash)).join("\n");
    assert!(rendered.contains("1 errors caught"), "{rendered}");
    assert!(
        rendered.contains("java.lang.NullPointerException"),
        "{rendered}"
    );
    assert!(rendered.contains("nothing at all"), "{rendered}");
}
