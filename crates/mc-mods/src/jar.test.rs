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

#[test]
fn le_side_d_une_dependance_est_retenu() {
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
fn un_jar_multi_mods_fusionne_les_cotes_d_une_meme_dependance() {
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

#[test]
fn union_des_cotes() {
    assert_eq!(Side::Client.union(Side::Client), Side::Client);
    assert_eq!(Side::Client.union(Side::Server), Side::Both);
    assert_eq!(Side::Both.union(Side::Client), Side::Both);
    assert!(Side::Both.includes(Side::Server));
    assert!(!Side::Client.includes(Side::Server));
}
