use super::{Before, LocalState, SCHEMA, decide, purge};
use crate::fixtures::Workshop;

fn state(generation: u32) -> LocalState {
    LocalState::new("aa".into(), generation, "2026-09-18T00:00:00Z".into())
}

// --- The decision, which is a pure function ---------------------------------

/// With no state, we know nothing of what's laid down: purge.
///
/// This is the case for EVERY machine already installed, the first time it
/// launches this version — one hundred percent of the population, once.
/// Nothing announces it today, and it should be noted in the release notes.
#[test]
fn with_no_state_we_purge() {
    assert_eq!(decide(None, 0), Before::Purge);
    assert_eq!(decide(None, 7), Before::Purge);
}

/// Same generation: nothing special, differential. This is the common case,
/// and the one that has to stay cheap — half a gigabyte doesn't get
/// redownloaded on every game session.
#[test]
fn at_equal_generation_we_install_by_diff() {
    assert_eq!(decide(Some(&state(0)), 0), Before::Differential);
    assert_eq!(decide(Some(&state(3)), 3), Before::Differential);
}

/// Requested generation higher: whoever published asked for a clean
/// reinstall.
#[test]
fn a_higher_generation_triggers_a_purge() {
    assert_eq!(decide(Some(&state(0)), 1), Before::Purge);
    assert_eq!(decide(Some(&state(3)), 4), Before::Purge);
    // A jump of several generations — a player who hasn't launched the
    // launcher in months — purges once, not once per generation.
    assert_eq!(decide(Some(&state(1)), 9), Before::Purge);
}

/// Requested generation LOWER: we don't purge.
///
/// It's the `<` rather than the `!=`, and this is the test that holds it in
/// place. Going back to an earlier generation means republishing a state that
/// was known good; erasing the installation on that occasion would punish the
/// player for an operations decision. The diff will put back the earlier
/// files.
#[test]
fn a_rollback_does_not_purge() {
    assert_eq!(decide(Some(&state(5)), 4), Before::Differential);
    assert_eq!(decide(Some(&state(5)), 0), Before::Differential);
}

// --- The state on disk -------------------------------------------------------

#[test]
fn a_written_state_reads_back_identically() {
    let workshop = Workshop::new("state-round-trip");
    let path = workshop.root.join("instances").join("x").join("state.json");

    let written = state(2);
    written.write(&path).expect("write");

    assert_eq!(LocalState::read(&path), Some(written));
}

/// An absent state isn't an error: it's "we don't know", and the behavior
/// that follows is the same as for an unreadable state.
#[test]
fn an_absent_state_returns_none() {
    let workshop = Workshop::new("state-absent");
    assert_eq!(LocalState::read(&workshop.root.join("nowhere.json")), None);
}

/// An unreadable state counts as absent, and doesn't fail the launch. The
/// opposite would prevent playing because of a corrupted file — an abrupt
/// stop mid-write, a full disk.
#[test]
fn a_broken_state_returns_none() {
    let workshop = Workshop::new("state-broken");
    let path = workshop.write("state.json", b"{this is not JSON");
    assert_eq!(LocalState::read(&path), None);
}

/// An unknown schema counts as absent, HENCE a purge.
///
/// This is the check the lockfile never had: its `schema` field is written
/// and never read back. Repeating that dead field in a new file would have
/// been deliberately making the mistake we're calling out.
#[test]
fn an_unknown_schema_counts_as_absent_hence_a_purge() {
    let workshop = Workshop::new("state-schema");
    let future = format!(
        r#"{{"schema":{},"lock_sha512":"aa","generation":3,"placed_at":"2026-09-18T00:00:00Z"}}"#,
        SCHEMA + 1
    );
    let path = workshop.write("state.json", future.as_bytes());

    let read_back = LocalState::read(&path);
    assert_eq!(read_back, None);
    assert_eq!(decide(read_back.as_ref(), 3), Before::Purge);
}

// --- The purge ----------------------------------------------------------------

/// What the purge erases, and above all what it does NOT erase.
///
/// The most important test in the module: losing a world to catch up on a mod
/// rename would be a cure worse than the disease.
#[test]
fn the_purge_only_erases_what_the_launcher_laid_down() {
    let workshop = Workshop::new("purge");
    let game = workshop.root.join("minecraft");

    // What the launcher lays down, and is allowed to erase.
    for erased in ["mods", "shaderpacks", "resourcepacks", "libraries"] {
        std::fs::create_dir_all(game.join(erased)).unwrap();
        std::fs::write(game.join(erased).join("one.jar"), b"x").unwrap();
    }
    // A subdirectory and a .jar.disabled: the exact leftover the diff doesn't
    // see, since the lockfile no longer describes it.
    std::fs::create_dir_all(game.join("mods").join("old")).unwrap();
    std::fs::write(game.join("mods").join("old.jar.disabled"), b"x").unwrap();

    // What belongs to the player, and nothing authorizes touching.
    for kept in ["saves", "config", "screenshots", "logs", "schematics"] {
        std::fs::create_dir_all(game.join(kept)).unwrap();
        std::fs::write(game.join(kept).join("precious"), b"x").unwrap();
    }
    std::fs::write(game.join("options.txt"), b"fullscreen:true\n").unwrap();

    let result = purge(&game);

    for erased in ["mods", "shaderpacks", "resourcepacks", "libraries"] {
        assert!(
            !game.join(erased).exists(),
            "{erased} should have been erased"
        );
    }
    for kept in ["saves", "config", "screenshots", "logs", "schematics"] {
        assert!(
            game.join(kept).join("precious").exists(),
            "{kept} was erased: an evening of reconfiguration, or a world"
        );
    }
    assert!(game.join("options.txt").exists());

    assert!(result.happened());
    assert_eq!(result.cleared.len(), 4);
    assert!(result.failures.is_empty(), "{:?}", result.failures);
}

/// `config/` is NOT erased, and it deserves its own test: it's the directory
/// where the player tunes their mods, and the only one whose removal would be
/// up for debate. A format change that truly required it calls for a human
/// decision, announced — not an incremented number in a file.
#[test]
fn the_purge_spares_the_mods_configuration() {
    let workshop = Workshop::new("purge-config");
    let game = workshop.root.join("minecraft");
    std::fs::create_dir_all(game.join("config").join("jei")).unwrap();
    std::fs::write(game.join("config").join("jei").join("jei.ini"), b"x").unwrap();

    purge(&game);

    assert!(game.join("config").join("jei").join("jei.ini").exists());
}

/// A purge on an instance with nothing to clear does nothing, and doesn't
/// report anything. This is the very first launch: announcing a purge to
/// someone installing for the first time would make them think something was
/// erased.
#[test]
fn a_purge_with_nothing_to_clear_reports_nothing() {
    let workshop = Workshop::new("purge-empty");
    let game = workshop.root.join("minecraft");
    std::fs::create_dir_all(&game).unwrap();

    let result = purge(&game);

    assert!(!result.happened());
    assert!(result.cleared.is_empty());
}

/// And a purge on a directory that doesn't exist at all doesn't panic.
#[test]
fn a_purge_on_nothing_at_all_does_not_panic() {
    let workshop = Workshop::new("purge-nothing");
    let result = purge(&workshop.root.join("never-created"));
    assert!(!result.happened());
}
