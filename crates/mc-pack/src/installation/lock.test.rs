use super::retain;
use crate::fixtures::{Workshop, entry, lock};
use crate::manifest::Manifest;

fn manifest() -> Manifest {
    Manifest::parse(crate::fixtures::MANIFEST.as_bytes()).unwrap()
}

/// Replaying a lock means obeying it, not rewriting it. Regenerating it
/// would wipe the `reason` column: everything in it would become
/// "requested by the manifest", since it's the lock itself that dictated
/// the requests, and the only trace of what had never been requested would
/// be lost.
#[test]
fn a_replayed_lock_is_left_as_is() {
    let workshop = Workshop::new("lock-replay");
    let path = workshop.root.join("samflix.lock.json");

    let mut previous = lock(vec![entry("bookshelf", "both", None)]);
    previous.mods[0].reason = "implicit dependency of jei".into();

    let retained = retain(
        &manifest(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        Some(&previous),
        &path,
        true,
    )
    .unwrap();

    assert_eq!(retained.mods.len(), 1);
    assert_eq!(retained.mods[0].reason, "implicit dependency of jei");
    // The loader version stays the one from the lock, not the one passed in.
    assert_eq!(retained.loader.version, "21.1.250");
    // Nothing is written: the replayed lock is already on disk.
    assert!(!path.exists());
}

/// Outside a replay, the lock describes what was just done, and it is
/// written.
#[test]
fn a_fresh_lock_is_written_with_the_placed_version() {
    let workshop = Workshop::new("lock-fresh");
    let path = workshop.root.join("samflix.lock.json");

    let retained = retain(
        &manifest(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        None,
        &path,
        false,
    )
    .unwrap();

    assert_eq!(retained.name, "samflix");
    assert_eq!(retained.loader.version, "21.1.999");
    assert_eq!(retained.loader.kind, "neoforge");
    assert_eq!(retained.java, 21);
    assert!(path.is_file(), "the lock was not written");
}

/// A replay requested without a lock to replay falls back to writing: it's
/// the only behavior that loses nothing.
#[test]
fn a_replay_without_a_lock_still_writes() {
    let workshop = Workshop::new("lock-replay-empty");
    let path = workshop.root.join("samflix.lock.json");

    let retained = retain(
        &manifest(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        None,
        &path,
        true,
    )
    .unwrap();

    assert_eq!(retained.loader.version, "21.1.999");
    assert!(path.is_file());
}
