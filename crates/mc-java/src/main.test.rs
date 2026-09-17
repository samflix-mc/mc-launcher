use super::{Reglages, analyser};
use std::path::PathBuf;

fn analyse(args: &[&str]) -> anyhow::Result<Reglages> {
    analyser(args.iter().map(|a| (*a).to_string()))
}

#[test]
fn sans_argument_on_cherche_un_java_21() {
    // Minecraft 1.21.1 l'exige ; en dessous le jeu s'arrête avant d'afficher
    // une fenêtre.
    assert_eq!(analyse(&[]).unwrap(), Reglages::default());
    assert_eq!(analyse(&[]).unwrap().major, 21);
}

#[test]
fn les_trois_options_se_combinent() {
    let reglages = analyse(&["--check", "--major", "17", "--dir", "/tmp/runtimes"]).unwrap();

    assert!(reglages.check_only);
    assert_eq!(reglages.major, 17);
    assert_eq!(reglages.dir, Some(PathBuf::from("/tmp/runtimes")));
}

/// Une valeur illisible est une faute de frappe, pas un bug : elle doit
/// s'annoncer comme une erreur ordinaire, avec ce qui a été lu.
#[test]
fn un_majeur_illisible_se_dit_sans_paniquer() {
    let erreur = analyse(&["--major", "vingt-et-un"]).expect_err("pas un entier");
    assert!(format!("{erreur:#}").contains("vingt-et-un"), "{erreur:#}");
}

#[test]
fn une_option_qui_attend_une_valeur_la_reclame() {
    assert!(analyse(&["--major"]).is_err());
    assert!(analyse(&["--dir"]).is_err());
}

#[test]
fn une_option_inconnue_est_nommee() {
    let erreur = analyse(&["--majeur"]).expect_err("option inconnue");
    assert!(format!("{erreur:#}").contains("--majeur"), "{erreur:#}");
}

/// `--check` ne touche à rien : il dit si ce poste a déjà un Java utilisable,
/// et rend l'échec quand il n'y en a pas — c'est ce qu'une CI appelle.
#[tokio::test]
async fn check_sans_runtime_rend_l_echec_sans_rien_installer() {
    let racine = std::env::temp_dir().join(format!("mc-java-main-{}", std::process::id()));
    std::fs::create_dir_all(&racine).unwrap();

    // Une version majeure qu'aucun système ne fournira, pour que le PATH du
    // poste ne vienne pas troubler le résultat.
    let code = super::executer(999, true, Some(racine.clone()))
        .await
        .unwrap();

    assert_eq!(
        format!("{code:?}"),
        format!("{:?}", std::process::ExitCode::FAILURE)
    );
    assert!(
        !racine.join("temurin-999").exists(),
        "rien ne doit être installé"
    );
    std::fs::remove_dir_all(&racine).ok();
}
