use super::retenir;
use crate::essais::{Atelier, entree, verrou};
use crate::manifest::Manifest;

fn manifeste() -> Manifest {
    Manifest::parse(crate::essais::MANIFESTE.as_bytes()).unwrap()
}

/// Rejouer un verrou, c'est lui obéir, pas le réécrire. Le régénérer effacerait
/// la colonne `reason` : tout y deviendrait « demandé par le manifeste »,
/// puisque c'est le verrou lui-même qui a dicté les demandes, et on perdrait la
/// seule trace de ce qui n'avait jamais été demandé.
#[test]
fn un_verrou_rejoue_est_laisse_tel_quel() {
    let atelier = Atelier::neuf("verrou-rejeu");
    let chemin = atelier.racine.join("samflix.lock.json");

    let mut precedent = verrou(vec![entree("bookshelf", "both", None)]);
    precedent.mods[0].reason = "dépendance implicite de jei".into();

    let retenu = retenir(
        &manifeste(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        Some(&precedent),
        &chemin,
        true,
    )
    .unwrap();

    assert_eq!(retenu.mods.len(), 1);
    assert_eq!(retenu.mods[0].reason, "dépendance implicite de jei");
    // La version du chargeur reste celle du verrou, pas celle qu'on passe.
    assert_eq!(retenu.loader.version, "21.1.250");
    // Rien n'est écrit : le verrou rejoué est déjà sur le disque.
    assert!(!chemin.exists());
}

/// Hors rejeu, le verrou décrit ce qui vient d'être fait, et il est écrit.
#[test]
fn un_verrou_neuf_est_ecrit_avec_la_version_posee() {
    let atelier = Atelier::neuf("verrou-neuf");
    let chemin = atelier.racine.join("samflix.lock.json");

    let retenu = retenir(
        &manifeste(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        None,
        &chemin,
        false,
    )
    .unwrap();

    assert_eq!(retenu.name, "samflix");
    assert_eq!(retenu.loader.version, "21.1.999");
    assert_eq!(retenu.loader.kind, "neoforge");
    assert_eq!(retenu.java, 21);
    assert!(chemin.is_file(), "le verrou n'a pas été écrit");
}

/// Un rejeu demandé sans verrou à rejouer retombe sur l'écriture : c'est le
/// seul comportement qui ne perde rien.
#[test]
fn un_rejeu_sans_verrou_ecrit_quand_meme() {
    let atelier = Atelier::neuf("verrou-rejeu-vide");
    let chemin = atelier.racine.join("samflix.lock.json");

    let retenu = retenir(
        &manifeste(),
        "21.1.999",
        21,
        &mc_mods::Plan::default(),
        None,
        &chemin,
        true,
    )
    .unwrap();

    assert_eq!(retenu.loader.version, "21.1.999");
    assert!(chemin.is_file());
}
