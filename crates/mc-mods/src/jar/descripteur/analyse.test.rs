use super::*;

#[test]
fn descripteur_neoforge_moderne() {
    let info = parse_descriptor(
        r#"
modLoader="javafml"
loaderVersion="[21,)"
license="MIT"

[[mods]]
modId="attributefix"
version="21.1.3"

[[dependencies.attributefix]]
modId="neoforge"
type="required"
versionRange="[21.1.65,)"

[[dependencies.attributefix]]
modId="bookshelf"
type="required"
versionRange="[21.1.0,)"

[[dependencies.attributefix]]
modId="prickle"
type="optional"
"#,
    )
    .unwrap();

    assert!(info.provides.contains("attributefix"));
    // neoforge est la plateforme, prickle est facultatif : reste bookshelf.
    assert_eq!(info.requires.len(), 1);
    assert_eq!(info.requires[0].mod_id, "bookshelf");
    assert_eq!(info.requires[0].version_range.as_deref(), Some("[21.1.0,)"));
}

#[test]
fn descripteur_forge_avec_mandatory() {
    let info = parse_descriptor(
        r#"
[[mods]]
modId="vieuxmod"

[[dependencies.vieuxmod]]
modId="jei"
mandatory=true

[[dependencies.vieuxmod]]
modId="jade"
mandatory=false
"#,
    )
    .unwrap();
    assert_eq!(info.requires.len(), 1);
    assert_eq!(info.requires[0].mod_id, "jei");
}
