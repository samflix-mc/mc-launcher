use super::jouer;
use crate::commandes::essais::{Atelier, entree, verrou};
use crate::commandes::lancement::annonce::annoncer;
use crate::commandes::lancement::preparation::Partie;
use mc_instance::launch::{Command, Session};

/// Une partie dont la JVM est `/bin/sh` : ce qui est vérifié n'est pas
/// Minecraft, mais ce que le launcher fait de sa sortie et de son verdict.
#[cfg(unix)]
fn partie(atelier: &Atelier, script: &str, cible: Option<&str>, explicite: bool) -> Partie {
    let options = atelier.options();
    let instance = options.layout.instance("samflix");
    instance.create().unwrap();

    Partie {
        instance,
        lock: verrou(vec![entree("jei", "both")]),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "0123456789abcdef"),
        cible: cible.map(str::to_string),
        demande_explicite: explicite,
        environnement: mc_log::Environment::Production,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn une_partie_qui_se_termine_bien_ne_remonte_rien() {
    let atelier = Atelier::neuf("partie-ok");
    atelier.pack_installe(vec![entree("jei", "both")]);

    jouer(&partie(
        &atelier,
        "echo 'Stopping worker threads'",
        None,
        false,
    ))
    .await
    .expect("sortie normale");
}

/// Fermer le jeu au clavier n'est pas une panne : le signaler comme telle
/// ouvrirait un incident à chaque partie terminée.
#[cfg(unix)]
#[tokio::test]
async fn une_partie_interrompue_n_est_pas_un_echec() {
    let atelier = Atelier::neuf("partie-interrompue");
    atelier.pack_installe(Vec::new());

    // 130 : ce qu'un shell rend pour un Ctrl+C.
    jouer(&partie(&atelier, "exit 130", None, false))
        .await
        .expect("une interruption n'est pas une erreur");
}

/// Minecraft rattrape beaucoup d'exceptions et continue : elles comptent
/// autant qu'un plantage, et ce sont elles qu'on ne verrait jamais autrement.
#[cfg(unix)]
#[tokio::test]
async fn les_erreurs_traversees_sont_remontees_meme_si_la_partie_finit_bien() {
    let atelier = Atelier::neuf("partie-erreurs");
    atelier.pack_installe(vec![entree("jei", "both")]);

    jouer(&partie(
        &atelier,
        "echo 'java.lang.NullPointerException: rien'; echo 'on continue'",
        None,
        false,
    ))
    .await
    .expect("la partie s'est bien terminée malgré l'exception");
}

/// Un arrêt sur erreur remonte, avec le répertoire des journaux du jeu : c'est
/// là qu'on ira chercher.
#[cfg(unix)]
#[tokio::test]
async fn un_arret_sur_erreur_nomme_les_journaux_du_jeu() {
    let atelier = Atelier::neuf("partie-echec");
    atelier.pack_installe(Vec::new());

    let erreur = jouer(&partie(&atelier, "exit 1", None, false))
        .await
        .expect_err("code 1");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("code 1"), "{texte}");
    assert!(texte.contains("logs"), "{texte}");
}

/// La provenance du serveur est dite, pas seulement l'adresse : quelqu'un qui
/// diagnostique une éjection a besoin de savoir s'il regarde le pack ou un
/// `--serveur`.
#[cfg(unix)]
#[test]
fn l_annonce_traverse_ses_trois_provenances() {
    let atelier = Atelier::neuf("annonce");
    atelier.pack_installe(Vec::new());

    annoncer(&partie(&atelier, "true", Some("mc.ggy.info"), false));
    annoncer(&partie(&atelier, "true", Some("essai.invalid"), true));
    annoncer(&partie(&atelier, "true", None, false));
}

/// Minecraft rattrape beaucoup d'exceptions et continue : ces erreurs-là
/// n'apparaissent nulle part ailleurs, et ce sont souvent elles qui expliquent
/// un comportement signalé bien plus tard. Annoncer « 0 erreurs » après une
/// partie sans incident ferait chercher une panne inexistante.
#[test]
fn les_erreurs_relevees_ne_s_annoncent_que_s_il_y_en_a() {
    assert!(super::lignes_d_erreurs(&[]).is_empty());

    let crash = mc_instance::crash::parse("java.lang.NullPointerException: rien du tout")
        .expect("une exception");

    let rendu = super::lignes_d_erreurs(std::slice::from_ref(&crash)).join("\n");
    assert!(rendu.contains("1 erreurs relevées"), "{rendu}");
    assert!(rendu.contains("java.lang.NullPointerException"), "{rendu}");
    assert!(rendu.contains("rien du tout"), "{rendu}");
}
