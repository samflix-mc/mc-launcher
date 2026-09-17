use super::ecarts;
use crate::essais::{entree, verrou};

/// Ce que le plan a réellement posé, sous la forme `(slug, build)`.
fn poses<'a>(mods: &'a [(&'a str, &'a str)]) -> impl Iterator<Item = (&'a str, &'a str)> {
    mods.iter().map(|(slug, build)| (*slug, *build))
}

#[test]
fn un_verrou_rejoue_a_l_identique_ne_signale_rien() {
    let attendu = verrou(vec![entree("jei", "both", None)]);
    // `entree` fabrique le build « <slug>-1.0 ».
    assert!(ecarts(&attendu, poses(&[("jei", "jei-1.0")])).is_empty());
}

#[test]
fn un_mod_du_verrou_qui_manque_est_nomme() {
    // Une source qui a retiré un build, une résolution qui n'a rien trouvé :
    // l'installation se termine « bien », et NeoForge refusera de démarrer.
    let attendu = verrou(vec![
        entree("jei", "both", None),
        entree("jade", "both", None),
    ]);

    let ecarts = ecarts(&attendu, poses(&[("jei", "jei-1.0")]));

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("jade"), "{ecarts:?}");
    assert!(ecarts[0].contains("absent"), "{ecarts:?}");
}

#[test]
fn un_build_qui_a_glisse_est_nomme_avec_les_deux_versions() {
    // Comparer les slugs seuls ne verrait pas ce cas — et c'est exactement
    // celui qui fait diverger un client de son serveur.
    let attendu = verrou(vec![entree("jei", "both", None)]);

    let ecarts = ecarts(&attendu, poses(&[("jei", "jei-2.0")]));

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("jei-1.0"), "{ecarts:?}");
    assert!(ecarts[0].contains("jei-2.0"), "{ecarts:?}");
}

#[test]
fn un_mod_en_trop_compte_autant_qu_un_mod_en_moins() {
    // NeoForge négocie ses registres à la connexion : un jar en trop fait
    // échouer la négociation aussi sûrement qu'un jar en moins.
    let attendu = verrou(vec![entree("jei", "both", None)]);

    let ecarts = ecarts(
        &attendu,
        poses(&[("jei", "jei-1.0"), ("sodium", "sodium-1.0")]),
    );

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("sodium"), "{ecarts:?}");
    assert!(ecarts[0].contains("absent du verrou"), "{ecarts:?}");
}

#[test]
fn les_ecarts_des_deux_sens_se_cumulent() {
    let attendu = verrou(vec![
        entree("jei", "both", None),
        entree("jade", "both", None),
    ]);

    let ecarts = ecarts(
        &attendu,
        poses(&[("jei", "jei-9.9"), ("sodium", "sodium-1.0")]),
    );

    assert_eq!(ecarts.len(), 3, "{ecarts:?}");
}

#[test]
fn un_verrou_vide_et_un_plan_vide_coincident() {
    assert!(ecarts(&verrou(Vec::new()), poses(&[])).is_empty());
}
