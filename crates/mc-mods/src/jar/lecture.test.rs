use super::parse_descriptor;
use crate::jar::Side;

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

/// Un jar sans rien d'embarqué n'apporte que lui-même.
#[test]
fn un_jar_ordinaire_ne_fournit_que_son_propre_modid() {
    let info = crate::jar::inspect_bytes(&crate::essais::jar("jei", &[])).unwrap();

    assert_eq!(info.provides.iter().collect::<Vec<_>>(), vec!["jei"]);
    assert!(info.bundled.is_empty());
}

/// La distinction que le bug de déduplication a rendue nécessaire : ce qu'un
/// jar embarque est un apport, pas une identité. Sodium et Iris embarquent les
/// mêmes shims Fabric ; les compter comme identité les faisait passer pour un
/// doublon, et supprimait l'un des deux en silence.
#[test]
fn un_modid_embarque_est_un_apport_et_non_une_identite() {
    let info = crate::jar::inspect_bytes(&crate::essais::jar_avec_embarque(
        "sodium",
        "fabric_api_base",
    ))
    .unwrap();

    assert_eq!(info.provides.iter().collect::<Vec<_>>(), vec!["sodium"]);
    assert_eq!(
        info.bundled.iter().collect::<Vec<_>>(),
        vec!["fabric_api_base"]
    );
    // Les deux ensembles réunis sont ce qui satisfait une dépendance.
    assert_eq!(info.fournit().count(), 2);
}

/// Un jar qu'on ne sait pas lire n'est pas une faute : il n'apporte rien, et
/// c'est tout ce qu'on peut en dire.
#[test]
fn un_jar_sans_descripteur_ne_fournit_rien() {
    let info =
        crate::jar::inspect_bytes(&crate::essais::archive(&[("README.txt", b"rien")])).unwrap();

    assert!(info.provides.is_empty());
    assert!(info.bundled.is_empty());
    assert!(info.requires.is_empty());
}
