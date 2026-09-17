use super::lignes;
use crate::commandes::essais::{Atelier, entree, verrou};
use crate::commandes::verify::lignes_non_resolues;

/// Le verrou est le produit de la commande : ce qu'on affiche est ce qu'une
/// revue lira. Le chargeur, le compte, puis chaque mod avec son côté et qui
/// l'a réclamé — c'est sur cette liste qu'on relit une résolution.
#[test]
fn le_rapport_nomme_le_chargeur_et_chaque_mod() {
    let atelier = Atelier::neuf("rapport");
    let chemin = atelier.racine.join("samflix.lock.json");
    let lock = verrou(vec![entree("jei", "both"), entree("sodium", "client")]);

    let rendu = lignes(&lock, &chemin, None).join("\n");

    assert!(rendu.contains("NeoForge 21.1.250"), "{rendu}");
    assert!(rendu.contains("2 mods"), "{rendu}");
    assert!(rendu.contains("jei"), "{rendu}");
    assert!(rendu.contains("sodium"), "{rendu}");
    assert!(rendu.contains("client"), "le côté manque : {rendu}");
    // Le chemin du verrou est la seule façon de le retrouver pour le relire.
    assert!(rendu.contains("samflix.lock.json"), "{rendu}");
}

/// Ce qu'une résolution a changé est ce qu'on vient vérifier. Le taire
/// laisserait croire qu'elle n'a rien fait alors qu'elle a remplacé dix
/// builds ; l'annoncer quand rien n'a bougé ferait chercher une différence
/// inexistante.
#[test]
fn les_changements_ne_paraissent_que_s_il_y_en_a() {
    let atelier = Atelier::neuf("rapport-diff");
    let chemin = atelier.racine.join("samflix.lock.json");
    let avant = verrou(vec![entree("jei", "both")]);
    let apres = verrou(vec![entree("jei", "both"), entree("sodium", "client")]);

    let inchange = lignes(&apres, &chemin, Some(&apres)).join("\n");
    assert!(
        !inchange.contains("Changements"),
        "des changements annoncés sans changement : {inchange}"
    );

    let modifie = lignes(&apres, &chemin, Some(&avant)).join("\n");
    assert!(modifie.contains("Changements"), "{modifie}");
    assert!(modifie.contains("sodium"), "{modifie}");
}

/// Une dépendance introuvable n'empêche pas d'écrire le verrou, mais empêchera
/// le jeu de démarrer : c'est le premier endroit à regarder, et il est nommé
/// avec qui l'exigeait.
#[test]
fn les_dependances_introuvables_sont_nommees_avec_leur_demandeur() {
    let mut lock = verrou(vec![entree("jei", "both")]);
    lock.unresolved.push(mc_pack::lockfile::LockedMissing {
        mod_id: "bookshelf".into(),
        required_by: "jei".into(),
        side: "both".into(),
    });

    let rendu = lignes_non_resolues(&lock).join("\n");

    assert!(rendu.contains("bookshelf"), "{rendu}");
    assert!(rendu.contains("jei"), "le demandeur manque : {rendu}");
    assert!(rendu.contains("refusera de démarrer"), "{rendu}");
}

/// Et rien du tout quand il n'y a rien à dire : un titre suivi d'une liste
/// vide ferait chercher une panne là où il n'y en a pas.
#[test]
fn sans_dependance_introuvable_rien_n_est_annonce() {
    let lock = verrou(vec![entree("jei", "both")]);
    assert!(lignes_non_resolues(&lock).is_empty());
}

/// Un verrou vide n'est pas une faute : c'est un pack sans mod, et la commande
/// doit s'en accommoder.
#[test]
fn un_verrou_sans_mod_s_annonce_quand_meme() {
    let atelier = Atelier::neuf("rapport-vide");
    let rendu = lignes(
        &verrou(Vec::new()),
        &atelier.racine.join("samflix.lock.json"),
        None,
    )
    .join("\n");

    assert!(rendu.contains("0 mods"), "{rendu}");
}
