use super::*;

#[test]
fn the_path_is_in_the_order_it_is_walked() {
    let ranks: Vec<usize> = Phase::ALL.iter().map(|phase| phase.rank()).collect();

    assert_eq!(ranks, (0..Phase::ALL.len()).collect::<Vec<_>>());
}

#[test]
fn mc_packs_steps_keep_their_order_once_translated() {
    // The seven installation steps must occupy seven consecutive ranks, and
    // in the same order: that's what lets the window advance one notch per
    // step received, without ever going backwards.
    let ranks: Vec<usize> = mc_pack::Step::ALL
        .iter()
        .map(|step| Phase::from(*step).rank())
        .collect();

    let first = ranks[0];
    assert_eq!(
        ranks,
        (first..first + mc_pack::Step::ALL.len()).collect::<Vec<_>>()
    );
}

#[test]
fn installation_is_framed_by_what_mc_pack_ignores() {
    // Sign-in and the license come before — mc-pack authenticates nothing —
    // and launch after. If either landed on the wrong side, the displayed
    // path would go backwards mid-way.
    assert!(Phase::SignIn.rank() < Phase::from(mc_pack::Step::Pack).rank());
    assert!(Phase::License.rank() < Phase::from(mc_pack::Step::Pack).rank());
    assert!(Phase::Ready.rank() > Phase::from(mc_pack::Step::Lock).rank());
    assert!(Phase::Launch.rank() > Phase::Ready.rank());
}

#[test]
fn each_phase_serializes_to_a_distinct_name() {
    // These names are the key on the TypeScript side: two phases sharing the
    // same one would light up only one of them.
    let names: std::collections::BTreeSet<String> = Phase::ALL
        .iter()
        .map(|phase| serde_json::to_string(phase).expect("serialization"))
        .collect();

    assert_eq!(names.len(), Phase::ALL.len());
}

#[test]
fn the_serialized_identifiers_do_not_move() {
    assert_eq!(
        serde_json::to_string(&Phase::NeoForge).expect("serialization"),
        "\"neo-forge\""
    );
    assert_eq!(
        serde_json::to_string(&Phase::SignIn).expect("serialization"),
        "\"signin\""
    );
}

#[test]
fn each_phase_has_a_non_empty_label() {
    // An empty label would leave a silent line in the displayed path.
    for phase in Phase::ALL {
        assert!(!phase.label().is_empty(), "{phase:?}");
    }
}
