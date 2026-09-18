use super::Side;
use crate::lockfile::reading::tests::{lock, locked};
use mc_mods::Origin;

#[test]
fn the_diff_says_what_moved() {
    let before = lock(vec![
        locked("jei", "a", "19.51"),
        locked("jade", "b", "15.10"),
    ]);
    let after = lock(vec![
        locked("jei", "c", "19.56"),
        locked("bookshelf-lib", "d", "21.1.81"),
    ]);

    let lines = after.diff(&before);
    assert!(lines.contains(&"~ jei 19.51 → 19.56".to_string()));
    assert!(lines.contains(&"+ bookshelf-lib 21.1.81".to_string()));
    assert!(lines.contains(&"- jade 15.10".to_string()));
}

#[test]
fn replaying_a_lockfile_pins_every_build() {
    let lockfile = lock(vec![locked("jei", "9myHusbW", "19.56")]);
    let requests = lockfile.requests();
    assert_eq!(requests[0].file.as_deref(), Some("9myHusbW"));
    assert_eq!(requests[0].source, Some(Origin::Modrinth));
    assert_eq!(requests[0].side, Some(Side::Both));
}

/// A generation that moves is spelled out, with its consequence.
///
/// It's the only line of the diff that describes an EFFECT and not a fact:
/// whoever publishes must see, in the PR review, that they just asked every
/// player to erase their mods. A lost “0 → 1” among thirty lines of
/// versions wouldn't tell them that.
#[test]
fn a_generation_that_moves_is_stated_with_its_consequence() {
    let mut before = lock(Vec::new());
    before.generation = 0;
    let mut after = lock(Vec::new());
    after.generation = 1;

    let lines = after.diff(&before);

    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("generation 0 → 1"), "{lines:?}");
    assert!(lines[0].contains("erase"), "{lines:?}");
    // The counterpart must be said in the same sentence: without it, the
    // line reads as “everything gets erased”, and nobody dares publish.
    assert!(lines[0].contains("saves"), "{lines:?}");
}

/// At constant generation, the diff says nothing about it — it only says
/// what moved.
#[test]
fn an_unchanged_generation_says_nothing() {
    let before = lock(Vec::new());
    let after = lock(Vec::new());
    assert!(after.diff(&before).is_empty());
}
