use super::{Placement, drifts};
use crate::fixtures::{entry, lock};
use mc_mods::Origin;

/// `entry` builds a Modrinth mod whose project is "<slug>-id" and build
/// "<slug>-1.0".
fn placement<'a>(project: &'a str, build: &'a str) -> Placement<'a> {
    (Origin::Modrinth, project, build)
}

#[test]
fn a_lock_replayed_identically_reports_nothing() {
    let expected = lock(vec![entry("jei", "both", None)]);

    assert!(drifts(&expected, [placement("jei-id", "jei-1.0")].into_iter()).is_empty());
}

/// The regression that filled the log with imaginary drifts: a CurseForge
/// mod is pinned under its readable slug, and requested again under its
/// numeric id. Compared by slug, these were two different mods — one
/// missing and one extra — for a single, perfectly compliant mod.
#[test]
fn a_project_requested_by_its_id_remains_the_same_mod() {
    let mut curseforge = entry("fix-gpu-memory-leak", "both", None);
    curseforge.source = Origin::CurseForge;
    curseforge.project = "882495".to_string();
    curseforge.file = "5513549".to_string();
    let expected = lock(vec![curseforge]);

    // The candidate carries the numeric id as its slug: that's what was
    // requested, and the project is nonetheless the same.
    let drifts = drifts(
        &expected,
        [(Origin::CurseForge, "882495", "5513549")].into_iter(),
    );

    assert!(drifts.is_empty(), "{drifts:?}");
}

#[test]
fn a_mod_missing_from_the_lock_is_named() {
    // A source that withdrew a build, a resolution that found nothing: the
    // installation finishes "fine", and NeoForge will refuse to start.
    let expected = lock(vec![
        entry("jei", "both", None),
        entry("jade", "both", None),
    ]);

    let drifts = drifts(&expected, [placement("jei-id", "jei-1.0")].into_iter());

    assert_eq!(drifts.len(), 1, "{drifts:?}");
    // Named by its slug, which is what a human recognizes.
    assert!(drifts[0].contains("jade"), "{drifts:?}");
    assert!(drifts[0].contains("missing"), "{drifts:?}");
}

#[test]
fn a_build_that_has_drifted_is_named_with_both_versions() {
    // Same project, different build: this is what makes a client diverge
    // from its server, and the only drift a project-only comparison would
    // miss.
    let expected = lock(vec![entry("jei", "both", None)]);

    let drifts = drifts(&expected, [placement("jei-id", "jei-2.0")].into_iter());

    assert_eq!(drifts.len(), 1, "{drifts:?}");
    assert!(drifts[0].contains("jei-1.0"), "{drifts:?}");
    assert!(drifts[0].contains("jei-2.0"), "{drifts:?}");
}

#[test]
fn an_extra_mod_counts_as_much_as_a_missing_one() {
    // NeoForge negotiates its registries at connect time: an extra jar
    // fails the negotiation just as surely as a missing one.
    let expected = lock(vec![entry("jei", "both", None)]);

    let drifts = drifts(
        &expected,
        [
            placement("jei-id", "jei-1.0"),
            placement("sodium-id", "sodium-1.0"),
        ]
        .into_iter(),
    );

    assert_eq!(drifts.len(), 1, "{drifts:?}");
    assert!(drifts[0].contains("sodium-id"), "{drifts:?}");
    assert!(drifts[0].contains("missing from lock"), "{drifts:?}");
}

/// Two sources can carry the same project id without it being the same
/// mod: the key is the pair, not the id alone.
#[test]
fn two_sources_are_not_confused_by_a_shared_id() {
    let expected = lock(vec![entry("jei", "both", None)]);

    let drifts = drifts(
        &expected,
        [(Origin::CurseForge, "jei-id", "jei-1.0")].into_iter(),
    );

    assert_eq!(drifts.len(), 2, "{drifts:?}");
}

#[test]
fn an_empty_lock_and_an_empty_plan_coincide() {
    let empty: [Placement<'_>; 0] = [];

    assert!(drifts(&lock(Vec::new()), empty.into_iter()).is_empty());
}

/// The real case that crashed the game: the manifest requests "sodium"
/// without a version and gets the latest build (0.8.13), while Iris
/// explicitly requires 0.6.13 for its compatibility mixins. The pack
/// installs, and Minecraft crashes on first connect on a missing class.
#[test]
fn an_unsatisfied_pinned_dependency_is_named() {
    let placements = [
        (Origin::Modrinth, "AANobbMI", "uMOpc5uV"), // sodium 0.8.13
        (Origin::Modrinth, "YL57xq9U", "t3ruzodq"), // iris 1.8.12
    ];

    let drifts = super::unsatisfied_dependencies(
        &placements,
        [super::Requirement {
            by: "iris",
            origin: Origin::Modrinth,
            project: "AANobbMI",
            build: "Pb3OXVqC", // sodium 0.6.13
        }]
        .into_iter(),
    );

    assert_eq!(drifts.len(), 1, "{drifts:?}");
    assert!(drifts[0].contains("iris"), "{drifts:?}");
    assert!(drifts[0].contains("Pb3OXVqC"), "{drifts:?}");
    assert!(drifts[0].contains("uMOpc5uV"), "{drifts:?}");
}

#[test]
fn a_satisfied_dependency_says_nothing() {
    let placements = [(Origin::Modrinth, "AANobbMI", "Pb3OXVqC")];

    let drifts = super::unsatisfied_dependencies(
        &placements,
        [super::Requirement {
            by: "iris",
            origin: Origin::Modrinth,
            project: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(drifts.is_empty(), "{drifts:?}");
}

/// A dependency missing from the pack is `Plan::unresolved`'s concern,
/// which already names it. Counting it here would make it appear twice.
#[test]
fn a_dependency_missing_from_the_pack_is_not_our_concern() {
    let placements: [Placement<'_>; 0] = [];

    let drifts = super::unsatisfied_dependencies(
        &placements,
        [super::Requirement {
            by: "iris",
            origin: Origin::Modrinth,
            project: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(drifts.is_empty(), "{drifts:?}");
}

/// A requirement on one source, the mod placed on another: it's not the
/// same project, and confusing them would invent a conflict.
#[test]
fn a_requirement_does_not_cross_sources() {
    let placements = [(Origin::CurseForge, "AANobbMI", "other")];

    let drifts = super::unsatisfied_dependencies(
        &placements,
        [super::Requirement {
            by: "iris",
            origin: Origin::Modrinth,
            project: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(drifts.is_empty(), "{drifts:?}");
}
