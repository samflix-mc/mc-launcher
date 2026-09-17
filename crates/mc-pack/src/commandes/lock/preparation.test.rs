use super::manifeste_a_verrouiller;
use crate::commandes::essais::{Atelier, MANIFESTE};
use mc_pack::source::Source;

/// Résoudre produit un verrou, et un verrou doit se poser quelque part. Un pack
/// publié n'offre pas cet endroit — et n'en a pas besoin : il arrive déjà
/// verrouillé, c'est justement ce qui en fait un pack publié.
#[test]
fn un_pack_publie_n_a_pas_a_etre_verrouille() {
    let atelier = Atelier::neuf("lock-distant");
    let source = Source::parse(
        "https://mc-launcher.ggy.info/pack/samflix.json",
        &atelier.options().layout,
    );

    let erreur = manifeste_a_verrouiller(&source).expect_err("rien à éditer");
    let texte = format!("{erreur:#}");
    assert!(texte.contains("donner un chemin"), "{texte}");
    assert!(texte.contains("mc-content"), "{texte}");
}

#[test]
fn un_manifeste_du_depot_est_accepte() {
    let atelier = Atelier::neuf("lock-local");
    let chemin = atelier.racine.join("samflix.json");
    std::fs::write(&chemin, MANIFESTE).unwrap();
    let source = Source::parse(chemin.to_str().unwrap(), &atelier.options().layout);

    let (rendu, manifeste) = manifeste_a_verrouiller(&source).expect("le manifeste est là");
    assert_eq!(rendu, chemin);
    assert_eq!(manifeste.name, "samflix");
}

/// C'est ici, et nulle part ailleurs, qu'une clé de « servers » illisible se
/// refuse : la lecture du manifeste s'applique aussi au pack téléchargé, et un
/// binaire qui refuserait un environnement inconnu s'arrêterait le jour où
/// mc-content en déclare un de plus.
#[test]
fn une_cle_de_serveur_qui_ne_serait_jamais_lue_est_refusee() {
    let atelier = Atelier::neuf("lock-cle");
    let chemin = atelier.racine.join("samflix.json");
    std::fs::write(
        &chemin,
        r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
            "loader":{"type":"neoforge","version":"21.1.250"},
            "servers":{"prodction":{"host":"mc.ggy.info"}}}"#,
    )
    .unwrap();
    let source = Source::parse(chemin.to_str().unwrap(), &atelier.options().layout);

    let erreur = manifeste_a_verrouiller(&source).expect_err("clé illisible");
    let texte = format!("{erreur:#}");
    assert!(texte.contains("prodction"), "{texte}");
    assert!(texte.contains("jamais lues"), "{texte}");
}

#[test]
fn un_manifeste_absent_se_dit_avec_son_chemin() {
    let atelier = Atelier::neuf("lock-absent");
    let chemin = atelier.racine.join("nulle-part.json");
    let source = Source::parse(chemin.to_str().unwrap(), &atelier.options().layout);

    let erreur = manifeste_a_verrouiller(&source).expect_err("rien à cette place");
    assert!(format!("{erreur:#}").contains("nulle-part"), "{erreur:#}");
}
