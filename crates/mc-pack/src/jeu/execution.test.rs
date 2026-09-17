//! Ce que le launcher fait de la sortie du jeu et de son verdict.
//!
//! La JVM des parties d'essai est `/bin/sh` : ce qui est vérifié n'est pas
//! Minecraft, mais l'interprétation de ce qu'un processus laisse derrière lui.

use super::{Partie, jouer};
use crate::essais::{Atelier, entree, verrou};
use mc_instance::launch::{Command, Outcome, Session};

#[cfg(unix)]
fn partie(atelier: &Atelier, script: &str) -> Partie {
    let options = atelier.options();
    let instance = options.layout.instance("samflix");
    instance.create().unwrap();

    Partie {
        instance,
        lock: verrou(vec![entree("jei", "both", None)]),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "0123456789abcdef"),
        cible: None,
        demande_explicite: false,
        environnement: mc_log::Environment::Production,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn une_partie_qui_se_termine_bien_rend_un_verdict_normal() {
    let atelier = Atelier::neuf("execution-ok");
    atelier.pack_installe(vec![entree("jei", "both", None)]);

    let rapport = jouer(&partie(&atelier, "echo 'Stopping worker threads'"))
        .await
        .expect("le jeu a bien été lancé");

    assert_eq!(rapport.outcome, Outcome::Normal);
    assert!(rapport.errors.is_empty());
}

/// Fermer le jeu au clavier n'est pas une panne : le signaler comme telle
/// ouvrirait un incident à chaque partie terminée.
#[cfg(unix)]
#[tokio::test]
async fn une_partie_interrompue_se_distingue_d_un_echec() {
    let atelier = Atelier::neuf("execution-interrompue");
    atelier.pack_installe(Vec::new());

    // 130 : ce qu'un shell rend pour un Ctrl+C.
    let rapport = jouer(&partie(&atelier, "exit 130"))
        .await
        .expect("une interruption n'est pas une erreur de lancement");

    assert!(matches!(rapport.outcome, Outcome::Interrupted { .. }));
}

/// Minecraft rattrape beaucoup d'exceptions et continue : elles comptent
/// autant qu'un plantage, et ce sont elles qu'on ne verrait jamais autrement.
#[cfg(unix)]
#[tokio::test]
async fn les_erreurs_traversees_sont_retenues_meme_si_la_partie_finit_bien() {
    let atelier = Atelier::neuf("execution-erreurs");
    atelier.pack_installe(vec![entree("jei", "both", None)]);

    let rapport = jouer(&partie(
        &atelier,
        "echo 'java.lang.NullPointerException: rien'; echo 'on continue'",
    ))
    .await
    .expect("la partie s'est bien terminée malgré l'exception");

    assert_eq!(rapport.outcome, Outcome::Normal);
    assert_eq!(rapport.errors.len(), 1, "{:?}", rapport.errors);
}

/// Un arrêt sur erreur **n'est pas** une erreur de cette fonction : le jeu a
/// bien été lancé. Remonter un `Err` obligerait la fenêtre à reconstituer le
/// code de sortie depuis un message.
#[cfg(unix)]
#[tokio::test]
async fn un_arret_sur_erreur_rend_son_code_sans_echouer() {
    let atelier = Atelier::neuf("execution-echec");
    atelier.pack_installe(Vec::new());

    let rapport = jouer(&partie(&atelier, "exit 1"))
        .await
        .expect("le lancement lui-même a réussi");

    assert_eq!(rapport.outcome, Outcome::Failed { code: 1 });
}

#[cfg(unix)]
#[test]
fn les_journaux_du_jeu_sont_nommes_dans_l_instance() {
    // C'est là qu'on envoie chercher quand une partie a mal tourné : un chemin
    // faux enverrait le joueur dans un dossier vide.
    let atelier = Atelier::neuf("execution-journaux");
    atelier.pack_installe(Vec::new());

    let chemin = super::journaux(&partie(&atelier, "true"));

    assert!(chemin.ends_with("logs"), "{}", chemin.display());
    assert!(chemin.starts_with(&atelier.racine), "{}", chemin.display());
}
