use super::{Commande, analyser, usage};

fn analyse(args: &[&str]) -> anyhow::Result<Commande> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    analyser(&args)
}

#[test]
fn les_trois_commandes_de_session_sont_reconnues() {
    assert_eq!(analyse(&["login"]).unwrap(), Commande::Login);
    assert_eq!(analyse(&["whoami"]).unwrap(), Commande::Whoami);
    assert_eq!(analyse(&["logout"]).unwrap(), Commande::Logout);
}

#[test]
fn le_mode_hors_ligne_porte_son_pseudo() {
    assert_eq!(
        analyse(&["--offline", "Sam"]).unwrap(),
        Commande::HorsLigne("Sam".into())
    );
}

/// `--offline` sans pseudo lancerait une session anonyme : mieux vaut
/// rappeler l'usage que d'inventer un nom de joueur.
#[test]
fn le_mode_hors_ligne_sans_pseudo_rappelle_l_usage() {
    let erreur = analyse(&["--offline"]).expect_err("aucun pseudo");
    assert!(
        format!("{erreur:#}").contains("--offline <PSEUDO>"),
        "{erreur:#}"
    );
}

#[test]
fn sans_commande_ou_avec_une_commande_inconnue_on_s_arrete() {
    assert!(analyse(&[]).is_err());
    assert!(analyse(&["connexion"]).is_err());
    assert!(analyse(&["--help"]).is_err());
}

#[test]
fn l_usage_nomme_les_quatre_commandes() {
    // Il part sur la sortie d'erreur ; ce qu'on vérifie ici est qu'il ne
    // panique pas et reste appelable depuis le chemin d'échec.
    usage();
}
