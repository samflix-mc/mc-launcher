use super::same_build;
use crate::resolve::fixtures::installed;

/// It's the version identifier that says whether two requests name the same
/// build — not the displayed number, which two sources can share. This
/// answer decides the replacement warning and impasse detection: getting it
/// backwards would announce replacements that aren't any.
#[test]
fn the_version_identifier_is_what_makes_it_the_same_build() {
    let in_place = installed("jei", &["jei"], &[]);

    let identical = in_place.candidate.clone();
    assert!(same_build(&in_place, &identical));

    let mut other = in_place.candidate.clone();
    other.version_id = "v2".into();
    assert!(!same_build(&in_place, &other));

    // Same displayed number, different build: this is the case of two
    // sources publishing the same version under different identifiers.
    let mut namesake = in_place.candidate.clone();
    namesake.version_id = "modrinth-xyz".into();
    assert_eq!(namesake.version_number, in_place.candidate.version_number);
    assert!(!same_build(&in_place, &namesake));
}
