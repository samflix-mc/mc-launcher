use super::{Side, inspect_all, missing_requirements};
use crate::resolve::fixtures::{bundling, installed, map};

#[test]
fn a_missing_dependency_is_reported() {
    let chosen = map(vec![installed(
        "attributefix",
        &["attributefix"],
        &[("bookshelf", Side::Both)],
    )]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "bookshelf");
    assert_eq!(missing[0].1, "attributefix");
}

#[test]
fn a_dependency_supplied_under_another_slug_does_not_go_missing() {
    // The modId `bookshelf` is published under the slug `bookshelf-lib`:
    // it's the jar's modId that counts, never the project's name.
    let chosen = map(vec![
        installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        ),
        installed("bookshelf-lib", &["bookshelf"], &[]),
    ]);
    assert!(missing_requirements(&chosen).is_empty());
}

#[test]
fn a_dependency_bundled_by_jarjar_does_not_go_missing() {
    // The mod bundles its library: installing it again would give two
    // versions of the same modId, which NeoForge refuses at load time.
    //
    // The bundled modId therefore satisfies the requirement, even though it
    // doesn't count as the mod's identity — that's the whole distinction.
    let chosen = map(vec![bundling(
        installed("a-mod", &["amod"], &[("some-lib", Side::Both)]),
        &["some-lib"],
    )]);
    assert!(missing_requirements(&chosen).is_empty());
}

/// What another mod bundles matters too: two mods from the same author
/// often share a library that only one of them carries.
#[test]
fn a_dependency_bundled_by_another_mod_does_not_go_missing_either() {
    let chosen = map(vec![
        installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        ),
        bundling(installed("another", &["other"], &[]), &["bookshelf"]),
    ]);
    assert!(missing_requirements(&chosen).is_empty());
}

/// Inspection only reads freshly downloaded jars: those that already
/// declare what they supply have been read, and those without a file don't
/// exist on disk yet. Confusing the two conditions would send `jar::inspect`
/// to open an empty path — resolution would stop on a read error, instead
/// of continuing its pass.
#[test]
fn an_entry_without_a_file_is_not_opened() {
    let mut chosen = map(vec![installed("not-yet", &[], &[])]);
    for entry in chosen.values_mut() {
        // Nothing downloaded: no path, and nothing declared.
        entry.path = std::path::PathBuf::new();
        entry.provides.clear();
    }

    inspect_all(&mut chosen).expect("an entry without a file is skipped");
}

#[test]
fn the_sides_of_two_requesters_are_merged() {
    let chosen = map(vec![
        installed("a", &["a"], &[("lib", Side::Client)]),
        installed("b", &["b"], &[("lib", Side::Server)]),
    ]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].2, Side::Both);
}
