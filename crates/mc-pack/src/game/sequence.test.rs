use super::{UpdateOutcome, should_catch_up};
use crate::comparison::{Action, Drift, PackState};

fn state(drift: Drift, offline: bool) -> PackState {
    PackState {
        action: Action::Play,
        drift,
        offline,
        installed: true,
        name: Some("samflix".into()),
        version: None,
        java: Some(21),
        mods: 0,
        generation: 0,
    }
}

/// The three drifts that call for action.
#[test]
fn what_has_moved_gets_caught_up() {
    for drift in [Drift::Absent, Drift::Update, Drift::Reinstall] {
        assert!(
            should_catch_up(&state(drift, false)),
            "{drift:?} should have triggered an installation"
        );
    }
}

/// Up to date: playing downloads nothing. It's the common case, and the one
/// that must stay instant — otherwise the single button would be a
/// regression for everyone except the day of an update.
#[test]
fn an_up_to_date_pack_triggers_nothing() {
    assert!(!should_catch_up(&state(Drift::UpToDate, false)));
}

/// Offline, we catch up on NOTHING, no matter what drift is shown.
///
/// `Drift::Unknown` doesn't mean "up to date", it means "we don't know".
/// Installing on that basis would start over from the cache to lay down
/// what's already there — several minutes of checksum verification,
/// learning nothing, at the exact moment the player has no network and just
/// wants to play.
#[test]
fn offline_we_catch_up_on_nothing() {
    for drift in [
        Drift::Unknown,
        Drift::Absent,
        Drift::Update,
        Drift::Reinstall,
        Drift::UpToDate,
    ] {
        assert!(
            !should_catch_up(&state(drift, true)),
            "{drift:?} triggered an installation while offline"
        );
    }
}

/// Without an installation, we don't claim to know what a resolution that
/// never happened would have found.
///
/// Returning an empty list rather than a stale one is what keeps the
/// "missing mods" banner from staying up after a launch where nothing was
/// resolved.
#[test]
fn without_an_installation_nothing_is_missing() {
    let outcome = UpdateOutcome {
        state: state(Drift::UpToDate, false),
        installation: None,
        session_report: None,
    };

    assert!(outcome.missing().is_empty());
    assert!(outcome.drifts().is_empty());
}

/// What an installation found MAKES IT UP to the report.
///
/// The neighboring test only covered the absence of an installation — so both
/// lists empty — and both accessors could return `vec![]` under any
/// circumstance without a test flinching. Yet they're what feeds the "missing
/// mods" banner and the drift fallback: silencing them would leave a player
/// playing with an incomplete pack with nothing telling them so, which is
/// exactly the situation a modded server launcher must prevent.
#[test]
fn what_an_installation_found_makes_it_up() {
    let mut lock = crate::fixtures::lock(Vec::new());
    lock.unresolved = vec![
        crate::fixtures::missing("jei", "the manifest"),
        crate::fixtures::missing("jade", "jei"),
    ];

    let outcome = UpdateOutcome {
        state: state(Drift::UpToDate, false),
        installation: Some(crate::Outcome {
            instance: mc_instance::Instance {
                name: "samflix".into(),
                dir: "/nowhere".into(),
                game_dir: "/nowhere/minecraft".into(),
            },
            server_dir: "/nowhere/server".into(),
            java: mc_java::Java {
                path: "/nowhere/java".into(),
                version: mc_java::Version {
                    major: 21,
                    full: "21.0.5".into(),
                },
                origin: mc_java::Origin::Managed,
            },
            neoforge: "21.1.250".into(),
            assets_downloaded: 0,
            libraries: 0,
            client_mods: 0,
            server_mods: 0,
            removed: Vec::new(),
            lock,
            lock_path: "/nowhere/samflix.lock.json".into(),
            previous_lock: None,
            source: "fixture".into(),
            from_cache: false,
            drifts: vec!["jei: 4.28.0 instead of 4.27.0".into()],
            purge: crate::state::Purge::default(),
        }),
        session_report: None,
    };

    assert_eq!(outcome.missing(), vec!["jei", "jade"]);
    assert_eq!(
        outcome.drifts(),
        vec!["jei: 4.28.0 instead of 4.27.0".to_string()]
    );
}
