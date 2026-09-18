use super::{Origin, Reason, deduplicate_by_mod_id};
use crate::resolve::fixtures::{bundling, installed, map};

#[test]
fn the_same_mod_from_two_sources_is_kept_only_once() {
    // Requested by its Modrinth slug, then pulled as a dependency by its
    // CurseForge identifier: nothing links the two project keys except the
    // modId. Two jars with the same modId would make NeoForge fail.
    let mut from_modrinth = installed("jade", &["jade"], &[]);
    from_modrinth.candidate.sha1 = Some("aa".into());

    let mut from_cf = installed("jade-cf", &["jade"], &[]);
    from_cf.candidate.origin = Origin::CurseForge;
    from_cf.candidate.sha1 = None;
    from_cf.reason = Reason::Declared { by: "other".into() };

    let mut chosen = map(vec![from_modrinth, from_cf]);
    let discarded = deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    // The one carrying a digest is kept: it's verifiable.
    assert!(chosen.values().next().unwrap().candidate.sha1.is_some());

    // What's removed is named, with the winner and the modId in question.
    assert_eq!(discarded.len(), 1);
    assert_eq!(discarded[0].discarded, "jade-cf");
    assert_eq!(discarded[0].kept, "jade");
    assert_eq!(discarded[0].mod_id, "jade");
    // A discarded dependency isn't a contradiction of the manifest.
    assert!(!discarded[0].explicit);
}

#[test]
fn at_equal_digest_the_requested_mod_wins() {
    let mut explicit = installed("jade", &["jade"], &[]);
    explicit.candidate.sha1 = Some("aa".into());

    let mut dependency = installed("jade-bis", &["jade"], &[]);
    dependency.candidate.sha1 = Some("bb".into());
    dependency.reason = Reason::Declared { by: "other".into() };

    let mut chosen = map(vec![explicit, dependency]);
    deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen.values().next().unwrap().reason, Reason::Explicit);
}

#[test]
fn two_distinct_mods_are_not_deduplicated() {
    let mut chosen = map(vec![
        installed("jei", &["jei"], &[]),
        installed("jade", &["jade"], &[]),
    ]);
    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2);
}

/// The bug that got Sodium, Iris and EntityCulling removed from the samflix
/// pack.
///
/// Sodium and Iris bundle the same four Fabric shims; EntityCulling and Not
/// Enough Animations the same tr7zw libs. These are contributions, not
/// identities: NeoForge deduplicates bundled jars at load time, and a pack
/// that keeps Iris without Sodium is broken.
#[test]
fn two_mods_bundling_the_same_library_both_remain() {
    let mut chosen = map(vec![
        bundling(
            installed("sodium", &["sodium"], &[]),
            &[
                "fabric_api_base",
                "fabric_block_view_api_v2",
                "fabric_renderer_api_v1",
            ],
        ),
        bundling(
            installed("iris", &["iris"], &[]),
            &[
                "fabric_api_base",
                "fabric_block_view_api_v2",
                "fabric_renderer_api_v1",
            ],
        ),
    ]);

    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2, "a legitimate mod was removed");
}

/// The boundary is really between root and bundled, not between "first" and
/// "second": a mod that bundles what another declares as its own stays
/// distinct from it.
#[test]
fn a_bundled_modid_does_not_take_the_spot_of_the_mod_that_declares_it() {
    let mut chosen = map(vec![
        // The library installed for itself.
        installed("cloth-config", &["cloth_config"], &[]),
        // A mod that bundles the same library.
        bundling(
            installed("architectury", &["architectury"], &[]),
            &["cloth_config"],
        ),
    ]);

    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2);
}

/// A mod requested in the manifest and discarded is flagged as such: that's
/// what lets the caller refuse instead of returning an incomplete lock.
#[test]
fn an_explicit_discarded_mod_is_flagged_as_such() {
    let mut first = installed("jade", &["jade"], &[]);
    first.candidate.sha1 = Some("aa".into());
    let mut second = installed("jade-mirror", &["jade"], &[]);
    second.candidate.sha1 = None;

    let mut chosen = map(vec![first, second]);
    let discarded = deduplicate_by_mod_id(&mut chosen);

    assert_eq!(discarded.len(), 1);
    assert_eq!(discarded[0].discarded, "jade-mirror");
    assert!(discarded[0].explicit, "both came from the manifest");
}
