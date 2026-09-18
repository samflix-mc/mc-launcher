use super::lines;
use crate::commands::fixtures::{Workshop, entry, lockfile};
use crate::commands::verify::unresolved_lines;

/// The lock is the command's product: what's shown is what a review will
/// read. The loader, the count, then each mod with its side and who
/// requested it — this is the list a resolution gets reread on.
#[test]
fn the_report_names_the_loader_and_each_mod() {
    let workshop = Workshop::new("report");
    let path = workshop.root.join("samflix.lock.json");
    let lock = lockfile(vec![entry("jei", "both"), entry("sodium", "client")]);

    let rendered = lines(&lock, &path, None).join("\n");

    assert!(rendered.contains("NeoForge 21.1.250"), "{rendered}");
    assert!(rendered.contains("2 mods"), "{rendered}");
    assert!(rendered.contains("jei"), "{rendered}");
    assert!(rendered.contains("sodium"), "{rendered}");
    assert!(
        rendered.contains("client"),
        "the side is missing: {rendered}"
    );
    // The lock's path is the only way to find it again to reread it.
    assert!(rendered.contains("samflix.lock.json"), "{rendered}");
}

/// What a resolution changed is what you come to check. Staying silent
/// would suggest it did nothing when it replaced ten builds; announcing it
/// when nothing moved would send someone looking for a nonexistent
/// difference.
#[test]
fn changes_only_appear_when_there_are_any() {
    let workshop = Workshop::new("report-diff");
    let path = workshop.root.join("samflix.lock.json");
    let before = lockfile(vec![entry("jei", "both")]);
    let after = lockfile(vec![entry("jei", "both"), entry("sodium", "client")]);

    let unchanged = lines(&after, &path, Some(&after)).join("\n");
    assert!(
        !unchanged.contains("Changes"),
        "changes announced without a change: {unchanged}"
    );

    let changed = lines(&after, &path, Some(&before)).join("\n");
    assert!(changed.contains("Changes"), "{changed}");
    assert!(changed.contains("sodium"), "{changed}");
}

/// A missing dependency doesn't stop the lock from being written, but it
/// will stop the game from starting: it's the first place to look, and it's
/// named with who requested it.
#[test]
fn missing_dependencies_are_named_with_their_requester() {
    let mut lock = lockfile(vec![entry("jei", "both")]);
    lock.unresolved.push(mc_pack::lockfile::LockedMissing {
        mod_id: "bookshelf".into(),
        required_by: "jei".into(),
        side: "both".into(),
    });

    let rendered = unresolved_lines(&lock).join("\n");

    assert!(rendered.contains("bookshelf"), "{rendered}");
    assert!(
        rendered.contains("jei"),
        "the requester is missing: {rendered}"
    );
    assert!(rendered.contains("refuse to start"), "{rendered}");
}

/// And nothing at all when there's nothing to say: a title followed by an
/// empty list would send someone looking for a problem that isn't there.
#[test]
fn without_a_missing_dependency_nothing_is_announced() {
    let lock = lockfile(vec![entry("jei", "both")]);
    assert!(unresolved_lines(&lock).is_empty());
}

/// An empty lock isn't a bug: it's a pack with no mods, and the command
/// must handle it.
#[test]
fn a_lock_with_no_mods_is_still_announced() {
    let workshop = Workshop::new("report-empty");
    let rendered = lines(
        &lockfile(Vec::new()),
        &workshop.root.join("samflix.lock.json"),
        None,
    )
    .join("\n");

    assert!(rendered.contains("0 mods"), "{rendered}");
}
