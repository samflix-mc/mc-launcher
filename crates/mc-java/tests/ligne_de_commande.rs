//! Le binaire tel qu'un script d'installation l'appelle.
//!
//! `executer` est vérifié dans le crate ; ce qui ne l'est pas, c'est ce que le
//! programme rend au shell. Or c'est toute son interface : la CI et les scripts
//! de lancement n'en lisent rien d'autre que le code de sortie, et une erreur
//! qui ressortirait en zéro ferait continuer un script sur un Java absent.

use std::path::Path;
use std::process::{Command, Output};

fn mc_java(args: &[&str], maison: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-java"))
        .args(args)
        .env("XDG_DATA_HOME", maison.join("donnees"))
        .env("HOME", maison)
        .env("SAMFLIX_TELEMETRY", "0")
        .output()
        .expect("le binaire mc-java a été construit par cargo test")
}

fn atelier(nom: &str) -> std::path::PathBuf {
    let racine = std::env::temp_dir().join(format!(
        "mc-java-cli-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&racine).ok();
    std::fs::create_dir_all(&racine).unwrap();
    racine
}

/// `--check` ne touche à rien et dit seulement si ce poste a le runtime. Une
/// version majeure que personne ne publie n'y est jamais : le code de sortie
/// doit le dire, sans quoi une CI enchaînerait sur un lancement voué à
/// l'échec.
#[test]
fn un_runtime_absent_se_dit_par_le_code_de_sortie() {
    let maison = atelier("check-absent");
    let sortie = mc_java(
        &[
            "--check",
            "--major",
            "999",
            "--dir",
            maison.join("runtimes").to_str().unwrap(),
        ],
        &maison,
    );

    assert_eq!(
        sortie.status.code(),
        Some(1),
        "sortie : {} / {}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    std::fs::remove_dir_all(&maison).ok();
}

/// Une option inconnue s'arrête avant d'installer quoi que ce soit, et le dit.
/// Un téléchargement de deux cents mégaoctets déclenché par une faute de frappe
/// serait une surprise coûteuse.
#[test]
fn une_option_inconnue_arrete_tout_et_se_nomme() {
    let maison = atelier("option-inconnue");
    let sortie = mc_java(&["--majeur", "21"], &maison);

    assert!(
        !sortie.status.success(),
        "une option inconnue ne peut pas réussir"
    );
    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreurs.contains("--majeur"), "{erreurs}");

    std::fs::remove_dir_all(&maison).ok();
}

/// `--major` attend un entier. Le refus doit nommer ce qui a été reçu : c'est
/// la seule façon de repérer une variable de shell vide passée sans guillemets.
#[test]
fn un_majeur_illisible_nomme_ce_qui_a_ete_recu() {
    let maison = atelier("majeur-illisible");
    let sortie = mc_java(&["--major", "vingt-et-un"], &maison);

    assert!(!sortie.status.success());
    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreurs.contains("vingt-et-un"), "{erreurs}");

    std::fs::remove_dir_all(&maison).ok();
}
