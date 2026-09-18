use super::{Reason, Request, authority, implicit_impasse};

/// These sentences are the only explanation a player gets when a mod they
/// didn't request appears in their pack, or when one build replaces
/// another. They must name who claimed what: an empty string, and the
/// report says nothing anymore.
#[test]
fn each_reason_says_who_claimed_the_mod() {
    assert_eq!(Reason::Explicit.describe(), "requested by the manifest");

    let declared = Reason::Declared {
        by: "create".to_string(),
    };
    assert_eq!(declared.describe(), "declared dependency of create");

    let implicit = Reason::Implicit {
        by: "create".to_string(),
        mod_id: "flywheel".to_string(),
    };
    assert_eq!(
        implicit.describe(),
        "implicit dependency: create requires \"flywheel\""
    );
}

/// The scenario that made resolution die on `MAX_PASSES`: an authoritative
/// requester imposes a build that doesn't supply the `modId` a jar
/// requires, and the requirement replayed its lost claim every pass.
#[test]
fn an_implicit_requirement_that_loses_its_spot_again_is_an_impasse() {
    let implicit = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(implicit_impasse(&implicit, 1, 3, false));
}

#[test]
fn an_implicit_requirement_that_wins_the_spot_is_not_an_impasse() {
    let implicit = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(!implicit_impasse(&implicit, 3, 1, false));
}

/// Losing the arbitration against a requester that names the same jar
/// deprives nothing: the `modId` will be supplied, the request no longer
/// has a reason to exist.
#[test]
fn losing_against_the_same_build_is_not_an_impasse() {
    let implicit = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(!implicit_impasse(&implicit, 1, 3, true));
}

/// A discarded declared dependency is pushed out by its parent when the
/// parent itself is replaced; it doesn't replay itself, so it doesn't need
/// to be recorded as an impasse.
#[test]
fn only_implicit_requirements_form_an_impasse() {
    let declared = Reason::Declared { by: "Jade".into() };
    assert!(!implicit_impasse(&declared, 1, 3, false));
    assert!(!implicit_impasse(&Reason::Explicit, 1, 3, false));
}

/// The manifest stays sovereign **as soon as it says something**: a request
/// it pins beats a pinned dependency.
#[test]
fn the_manifest_has_authority_when_it_pins_too() {
    let mut from_manifest = Request::new("sodium");
    from_manifest.file = Some("pack-choice".into());
    let mut from_dependency = Request::new("sodium");
    from_dependency.file = Some("iris-choice".into());

    assert!(
        authority(&Reason::Explicit, &from_manifest)
            > authority(&Reason::Declared { by: "iris".into() }, &from_dependency)
    );
}

/// The opposite of what the old rule did, and the reason for the change: a
/// request without a version expresses no version preference. Writing
/// "sodium" says "I want this mod," not "I want its latest version whatever
/// the cost" — and letting it override Iris's precise requirement made
/// Minecraft crash on the first connection.
#[test]
fn a_pinned_dependency_has_authority_over_a_versionless_request() {
    let mut from_dependency = Request::new("sodium");
    from_dependency.file = Some("Pb3OXVqC".into());

    assert!(
        authority(&Reason::Declared { by: "iris".into() }, &from_dependency)
            > authority(&Reason::Explicit, &Request::new("sodium"))
    );
}

/// Even a requirement read from a jar, that nothing announced: it knows
/// more about the version it needs than a request that says nothing about
/// it.
#[test]
fn any_pinned_request_outranks_an_open_request() {
    let mut pinned = Request::new("lib");
    pinned.file = Some("abc".into());
    let implicit = Reason::Implicit {
        by: "create".into(),
        mod_id: "flywheel".into(),
    };

    assert!(authority(&implicit, &pinned) > authority(&Reason::Explicit, &Request::new("lib")));
}

#[test]
fn at_equal_origin_pinning_has_authority() {
    let mut pinned = Request::new("jade");
    pinned.file = Some("abc".into());
    let reason = Reason::Declared { by: "x".into() };

    assert!(authority(&reason, &pinned) > authority(&reason, &Request::new("jade")));

    // A version number pins just as much as a build identifier.
    let mut by_version = Request::new("jade");
    by_version.version = Some("1.2.3".into());
    assert_eq!(authority(&reason, &by_version), authority(&reason, &pinned));
}

#[test]
fn a_declared_dependency_has_authority_over_an_implicit_dependency() {
    assert!(
        authority(&Reason::Declared { by: "x".into() }, &Request::new("lib"))
            > authority(
                &Reason::Implicit {
                    by: "x".into(),
                    mod_id: "lib".into()
                },
                &Request::new("lib")
            )
    );
}
