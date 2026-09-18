use super::{Arbitration, Side, arbitrate, confront, replacement_message};

/// The case that makes resolution converge: a declared dependency never
/// dislodges what the manifest has pinned.
#[test]
fn a_less_authoritative_request_dislodges_no_one() {
    assert_eq!(
        arbitrate(1, 3, false, Side::Both, Side::Client),
        Arbitration::Keep {
            side: Side::Both,
            takes_over: false
        }
    );
}

/// Same build on both sides: nothing to re-download, nothing to rewrite.
/// The more authoritative requester takes over the spot, so the lock names
/// whoever actually answers for the mod's presence.
#[test]
fn the_same_build_does_not_fight_over_itself_but_changes_requester() {
    assert_eq!(
        arbitrate(3, 1, true, Side::Server, Side::Server),
        Arbitration::Keep {
            side: Side::Server,
            takes_over: true
        }
    );
}

#[test]
fn a_more_authoritative_requester_imposes_its_build() {
    assert_eq!(
        arbitrate(3, 1, false, Side::Server, Side::Client),
        Arbitration::Replace { side: Side::Both }
    );
}

/// Claimed server-side by one branch, client-side by another: it must end
/// up on both sides, whichever one wins on the version.
#[test]
fn the_side_always_covers_both_uses() {
    for same_build in [true, false] {
        let side = match arbitrate(2, 2, same_build, Side::Client, Side::Server) {
            Arbitration::Keep { side, .. } | Arbitration::Replace { side } => side,
        };
        assert_eq!(side, Side::Both, "same build: {same_build}");
    }
}

/// At equal authority, whoever arrived first stays the one that answers for
/// the mod. Giving up the spot on a tie would make the name recorded in the
/// lock depend on the order the branches were explored in — two resolutions
/// of the same manifest would give two different locks.
#[test]
fn at_equal_authority_the_requester_in_place_stays() {
    assert_eq!(
        arbitrate(2, 2, false, Side::Client, Side::Client),
        Arbitration::Keep {
            side: Side::Client,
            takes_over: false
        }
    );
    // One notch above, on the other hand, the takeover is due.
    assert_eq!(
        arbitrate(3, 2, true, Side::Client, Side::Client),
        Arbitration::Keep {
            side: Side::Client,
            takes_over: true
        }
    );
}

/// The warning must name both versions and who decided: without that, the
/// player reads "a version was replaced," which is indistinguishable from
/// silence — and the pack doesn't have the version the manifest promises.
#[test]
fn the_warning_names_both_versions_and_the_reason() {
    use crate::resolve::fixtures::installed;
    use crate::resolve::reason::Reason;

    let discarded = installed("jei", &["jei"], &[]);
    let mut kept = discarded.candidate.clone();
    kept.version_number = "19.56".into();

    let message = replacement_message(&kept, &discarded, &Reason::Explicit);

    assert!(message.contains("jei"), "{message}");
    assert!(
        message.contains("19.56"),
        "the kept build isn't named: {message}"
    );
    assert!(
        message.contains("1.0"),
        "the discarded build isn't named: {message}"
    );
    assert!(
        message.contains("manifest"),
        "the reason is missing: {message}"
    );
}

/// `confront` applies the arbitration to the entry in place. Always
/// returning "nothing to do" would let the least authoritative build
/// install — and the manifest's pin wouldn't be worth anything anymore.
#[test]
fn confront_returns_the_side_to_record_when_the_build_is_replaced() {
    use crate::resolve::fixtures::installed;
    use crate::resolve::reason::Reason;

    let mut in_place = installed("jei", &["jei"], &[]);
    in_place.authority = 1;
    in_place.side = Side::Client;
    let mut incoming = in_place.candidate.clone();
    incoming.version_id = "v2".into();
    incoming.version_number = "19.56".into();

    let outcome = confront(
        &mut in_place,
        &incoming,
        &Reason::Explicit,
        4,
        Side::Server,
        false,
    );

    assert_eq!(
        outcome,
        Some(Side::Both),
        "the merged side must be recorded"
    );

    // And conversely, a less authoritative requester dislodges no one.
    let mut in_place = installed("jei", &["jei"], &[]);
    in_place.authority = 4;
    assert_eq!(
        confront(
            &mut in_place,
            &incoming,
            &Reason::Explicit,
            1,
            Side::Both,
            false
        ),
        None
    );
}
