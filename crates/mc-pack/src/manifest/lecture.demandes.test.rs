//! Ce qu'un manifeste lu redemande au résolveur.

use super::{Manifest, ModEntry};
use mc_mods::{Channel, Origin, Side};

use crate::manifest::essais::base;

#[test]
fn le_java_du_manifeste_prime_sur_celui_de_mojang() {
    let mut m = base();
    assert_eq!(m.java_major(21), 21);
    m.java = Some(22);
    assert_eq!(m.java_major(21), 22);
}

#[test]
fn le_cote_est_lu_depuis_le_texte() {
    let entry = ModEntry {
        slug: "embeddium".into(),
        source: None,
        file: None,
        version: None,
        side: Some("client".into()),
        channel: None,
    };
    assert_eq!(entry.to_request().unwrap().side, Some(Side::Client));
}

#[test]
fn un_cote_inconnu_est_refuse() {
    let entry = ModEntry {
        slug: "x".into(),
        source: None,
        file: None,
        version: None,
        side: Some("les-deux".into()),
        channel: None,
    };
    assert!(entry.to_request().is_err());
}

#[test]
fn aller_retour_json() {
    let json = r#"{
        "schema": 1,
        "name": "samflix",
        "minecraft": "1.21.1",
        "loader": { "type": "neoforge", "version": "latest" },
        "mods": [
            { "slug": "jei" },
            { "slug": "jade", "file": "eYz2YBGT", "source": "modrinth" },
            { "slug": "attributefix", "side": "both", "channel": "beta" }
        ]
    }"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    manifest.check().unwrap();
    assert!(manifest.loader.is_latest());
    assert_eq!(manifest.mods.len(), 3);
    assert_eq!(manifest.mods[1].source, Some(Origin::Modrinth));
    assert_eq!(manifest.mods[2].channel, Some(Channel::Beta));
}
