use super::*;

fn lire(args: &[&str]) -> Arguments {
    Arguments::lire(args.iter().map(|s| s.to_string()))
        .expect("arguments valides")
        .expect("une commande")
}

/// Sans commande, l'appelant doit afficher l'aide plutôt que de deviner.
#[test]
fn sans_commande_il_n_y_a_rien_a_faire() {
    let vide: Vec<String> = Vec::new();
    assert!(Arguments::lire(vide.into_iter())
        .expect("pas une erreur")
        .is_none());
}

/// Un chemin nu est la source : c'est la seule position qui ne porte pas de
/// nom, et la confondre avec une option ferait travailler sur le mauvais pack.
#[test]
fn le_chemin_nu_designe_le_pack() {
    let lu = lire(&["lock", "packs/samflix.json"]);
    assert_eq!(lu.command, "lock");
    assert_eq!(lu.source_arg.as_deref(), Some("packs/samflix.json"));
}

#[test]
fn les_options_a_valeur_prennent_le_mot_suivant() {
    let lu = lire(&["launch", "--pseudo", "Sam", "--memoire", "4096"]);
    assert_eq!(lu.pseudo.as_deref(), Some("Sam"));
    assert_eq!(lu.memoire, Some(4096));
}

/// Une option inconnue s'arrête ici plutôt que d'être ignorée : une faute de
/// frappe sur `--pseudo` lancerait sinon le jeu sous un autre nom.
#[test]
fn une_option_inconnue_arrete_tout() {
    let erreur = Arguments::lire(["launch", "--psuedo", "Sam"].iter().map(|s| s.to_string()))
        .expect_err("option inconnue");
    assert!(erreur.to_string().contains("--psuedo"), "{erreur}");
}
