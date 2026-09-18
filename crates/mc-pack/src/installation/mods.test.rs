use super::added_by_dependency;
use mc_mods::Reason;

/// This is the number that explains how a thirty-line manifest installs a
/// hundred mods: the rest comes from dependencies. Counting the others —
/// the ones the manifest names — would say the opposite, and suggest a
/// resolution that found nothing when it actually found everything.
#[test]
fn only_mods_not_requested_count_as_added() {
    let reasons = [
        Reason::Explicit,
        Reason::Declared {
            by: "jei".to_string(),
        },
        Reason::Implicit {
            by: "jei".to_string(),
            mod_id: "bookshelf".to_string(),
        },
    ];

    assert_eq!(added_by_dependency(reasons.iter()), 2);

    // A pack whose manifest names everything gained nothing along the way.
    assert_eq!(added_by_dependency([Reason::Explicit].iter()), 0);
    // And an empty pack added nothing either.
    assert_eq!(added_by_dependency([].iter()), 0);
}
