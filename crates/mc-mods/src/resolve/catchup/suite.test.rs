use super::{BTreeSet, CatchUp, Reason, Side, Unresolved, catch_up_outcome};
use crate::Origin;
use crate::resolve::fixtures::candidate;

/// declared, we find who supplies it, and its request joins the queue.
#[test]
fn a_found_supplier_becomes_an_implicit_request() {
    let outcome = catch_up_outcome(
        Some(candidate("bookshelf-lib", "21.1.81")),
        &BTreeSet::new(),
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    match outcome {
        CatchUp::Enqueue(request, reason) => {
            assert_eq!(request.slug, "bookshelf-lib-id");
            assert_eq!(request.source, Some(Origin::Modrinth));
            assert_eq!(request.file.as_deref(), Some("bookshelf-lib-21.1.81"));
            assert_eq!(
                reason,
                Reason::Implicit {
                    by: "attributefix".into(),
                    mod_id: "bookshelf".into()
                }
            );
        }
        other => panic!("expected a request, got {other:?}"),
    }
}

/// The case that made resolution run until MAX_PASSES: the only supplier
/// holds a key a more authoritative request already claims. Asking again
/// would change nothing, so the gap is recorded instead.
#[test]
fn a_supplier_in_an_impasse_is_recorded_instead_of_being_asked_again() {
    let mut impasses = BTreeSet::new();
    impasses.insert((Origin::Modrinth, "bookshelf-lib-id".to_string()));
    let outcome = catch_up_outcome(
        Some(candidate("bookshelf-lib", "21.1.81")),
        &impasses,
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    assert_eq!(
        outcome,
        CatchUp::GiveUp(Unresolved {
            mod_id: "bookshelf".into(),
            required_by: "attributefix".into(),
            side: Side::Both,
        })
    );
}

#[test]
fn a_modid_nobody_supplies_is_recorded() {
    let outcome = catch_up_outcome(
        None,
        &BTreeSet::new(),
        "libfoo".into(),
        "build_b".into(),
        Side::Client,
    );
    assert_eq!(
        outcome,
        CatchUp::GiveUp(Unresolved {
            mod_id: "libfoo".into(),
            required_by: "build_b".into(),
            side: Side::Client,
        })
    );
}
