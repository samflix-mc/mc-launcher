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
