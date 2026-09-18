use super::parse_descriptor;
use crate::jar::Side;

#[test]
fn a_dependency_s_side_is_kept() {
    let info = parse_descriptor(
        r#"
[[mods]]
modId="skin"

[[dependencies.skin]]
modId="embeddium"
type="required"
side="CLIENT"
"#,
    )
    .unwrap();
    assert_eq!(info.requires[0].side, Side::Client);
}

#[test]
fn a_multi_mod_jar_merges_the_sides_of_a_shared_dependency() {
    let info = parse_descriptor(
        r#"
[[mods]]
modId="a"
[[mods]]
modId="b"

[[dependencies.a]]
modId="lib"
type="required"
side="CLIENT"

[[dependencies.b]]
modId="lib"
type="required"
side="SERVER"
"#,
    )
    .unwrap();
    assert_eq!(info.provides.len(), 2);
    assert_eq!(info.requires.len(), 1);
    assert_eq!(info.requires[0].side, Side::Both);
}

/// A jar with nothing bundled brings only itself.
#[test]
fn a_plain_jar_provides_only_its_own_modid() {
    let info = crate::jar::inspect_bytes(&crate::fixtures::jar("jei", &[])).unwrap();

    assert_eq!(info.provides.iter().collect::<Vec<_>>(), vec!["jei"]);
    assert!(info.bundled.is_empty());
}

/// The distinction the deduplication bug made necessary: what a jar bundles
/// is a contribution, not an identity. Sodium and Iris bundle the same
/// Fabric shims; counting them as identity made them look like a duplicate,
/// and silently dropped one of the two.
#[test]
fn a_bundled_modid_is_a_contribution_and_not_an_identity() {
    let info = crate::jar::inspect_bytes(&crate::fixtures::jar_with_bundled(
        "sodium",
        "fabric_api_base",
    ))
    .unwrap();

    assert_eq!(info.provides.iter().collect::<Vec<_>>(), vec!["sodium"]);
    assert_eq!(
        info.bundled.iter().collect::<Vec<_>>(),
        vec!["fabric_api_base"]
    );
    // Both sets combined are what satisfies a dependency.
    assert_eq!(info.provided().count(), 2);
}

/// A jar we can't read isn't an error: it brings nothing, and that's all
/// there is to say about it.
#[test]
fn a_jar_without_a_descriptor_provides_nothing() {
    let info = crate::jar::inspect_bytes(&crate::fixtures::archive(&[("README.txt", b"nothing")]))
        .unwrap();

    assert!(info.provides.is_empty());
    assert!(info.bundled.is_empty());
    assert!(info.requires.is_empty());
}
