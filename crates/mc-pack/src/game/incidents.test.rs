use super::{report_game_crash, report_game_error};
use crate::fixtures::{Workshop, entry, lock};

const TRACE: &str = "\
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source)";

/// The launch, moved back a second: a file's write timestamp is less precise
/// than the clock, and the game runs for minutes between the two.
fn launch() -> std::time::SystemTime {
    std::time::SystemTime::now() - std::time::Duration::from_secs(1)
}

/// The launcher and the game are two processes: the Java trace stays on the
/// game's side, and that's the only moment it's available. No Sentry client
/// is initialized here — what's being checked is the collection, not the
/// sending.
#[test]
fn a_game_crash_is_picked_up_from_the_game_sessions_logs() {
    let workshop = Workshop::new("incident-crash");
    workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");

    let start = launch();
    let logs = instance.game_dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), TRACE).unwrap();

    report_game_crash(
        &instance,
        &lock(vec![entry("jei", "both", None)]),
        "neoforge-21.1.250",
        start,
        1,
    );
}

/// No usable trace: the launcher's incident stands, and it at least says
/// where to look. This path must not panic — it's the one for a player whose
/// game vanished without writing anything.
#[test]
fn a_crash_with_no_trace_does_not_panic_the_launcher() {
    let workshop = Workshop::new("incident-no-trace");
    workshop.installed_pack(Vec::new());
    let instance = workshop.options().layout.instance("samflix");

    report_game_crash(
        &instance,
        &lock(Vec::new()),
        "neoforge-21.1.250",
        launch(),
        1,
    );
}

/// The context attached is what we'd otherwise ask the player for: versions,
/// mods present, and where the trace comes from.
#[test]
fn a_caught_exception_is_distinguished_from_a_fatal_error() {
    let workshop = Workshop::new("incident-context");
    workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");

    let crash = mc_instance::crash::parse(TRACE).expect("an exception");
    let lock = lock(vec![entry("jei", "both", None)]);

    let mut id = None;
    let events = sentry::test::with_captured_events(|| {
        id = Some(report_game_error(
            &instance,
            &lock,
            "neoforge-21.1.250",
            &crash,
            Some(1),
        ));
    });

    assert_eq!(events.len(), 1, "{events:?}");
    assert_ne!(
        id.unwrap(),
        sentry::types::Uuid::nil(),
        "no id to give the player"
    );

    // The context attached is what we'd otherwise ask the player for,
    // question by question.
    let extra = &events[0].extra;
    assert_eq!(extra["version"].as_str(), Some("neoforge-21.1.250"));
    assert_eq!(extra["minecraft"].as_str(), Some("1.21.1"));
    assert_eq!(extra["neoforge"].as_str(), Some("21.1.250"));
    assert_eq!(extra["exit_code"].as_str(), Some("1"));
    assert!(
        extra["mods"].as_str().unwrap().contains("jei"),
        "the mod list is missing: {extra:?}"
    );

    // Without an exit code, the incident is distinguished from a fatal
    // error: otherwise the two would blend together in the dashboard.
    let caught = sentry::test::with_captured_events(|| {
        report_game_error(&instance, &lock, "neoforge-21.1.250", &crash, None);
    });
    assert!(
        !caught[0].extra.contains_key("exit_code"),
        "{:?}",
        caught[0].extra
    );
}

/// A crash with a trace becomes a full incident: it's the only moment we
/// have the exception, and a player will think to neither find it nor
/// attach it.
#[test]
fn a_crash_with_a_trace_is_sent_as_an_incident() {
    let workshop = Workshop::new("incident-send");
    workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");

    let start = launch();
    let logs = instance.game_dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), TRACE).unwrap();

    let events = sentry::test::with_captured_events(|| {
        report_game_crash(
            &instance,
            &lock(vec![entry("jei", "both", None)]),
            "neoforge-21.1.250",
            start,
            1,
        );
    });

    assert_eq!(events.len(), 1, "{events:?}");
    let exception = events[0]
        .exception
        .values
        .first()
        .expect("the trace is attached as an exception");
    assert_eq!(exception.ty, "java.lang.module.ResolutionException");
}

/// Without a trace, no game incident is sent: there's no exception to group,
/// and an empty incident would clutter the dashboard without teaching
/// anything.
#[test]
fn a_crash_with_no_trace_sends_no_game_incident() {
    let workshop = Workshop::new("incident-silent");
    workshop.installed_pack(Vec::new());
    let instance = workshop.options().layout.instance("samflix");

    let events = sentry::test::with_captured_events(|| {
        report_game_crash(
            &instance,
            &lock(Vec::new()),
            "neoforge-21.1.250",
            launch(),
            1,
        );
    });

    assert!(events.is_empty(), "{events:?}");
}
