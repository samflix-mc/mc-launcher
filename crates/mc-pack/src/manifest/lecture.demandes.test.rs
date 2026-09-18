//! Ce qu'un manifeste lu redemande au résolveur.

use super::{Manifest, ModEntry};
use mc_mods::{Channel, Origin, Side};

use crate::manifest::essais::base;

#[test]
fn le_java_du_manifeste_prime_sur_celui_de_mojang() {
    let mut m = base();
    assert_eq!(m.java_major(Some(21)), 21);
    m.java = Some(22);
    assert_eq!(m.java_major(Some(21)), 22);
}

/// Les descripteurs d'avant la 1.17 n'ont pas de bloc `javaVersion`. Le
/// manifeste du pack reste alors la seule source, et à défaut le 21 — le seul
/// endroit du depot ou ce chiffre est ecrit en dur.
#[test]
fn sans_exigence_de_mojang_le_manifeste_ou_le_defaut() {
    let mut m = base();
    assert_eq!(m.java_major(None), 21);
    m.java = Some(17);
    assert_eq!(m.java_major(None), 17);
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

    // Ce que le manifeste demande est ce que le résolveur ira chercher : une
    // liste vide installerait un pack sans mods, sans qu'aucune erreur ne le
    // signale — le jeu démarrerait, nu.
    let demandes = manifest.requests().expect("le manifeste est cohérent");
    assert_eq!(demandes.len(), 3, "{demandes:?}");
    assert_eq!(demandes[1].source, Some(Origin::Modrinth));
    assert_eq!(demandes[2].channel, Some(Channel::Beta));
}
