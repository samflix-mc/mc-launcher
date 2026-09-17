//! Ce que la ligne de commande écrit, et rien d'autre.
//!
//! La préparation et l'exécution sont vérifiées dans la bibliothèque, où elles
//! vivent. Ce qui reste ici est l'affichage — le récapitulatif avant de lancer,
//! et le compte rendu des erreurs relevées.

use super::super::essais::{Atelier, entree, verrou};
use super::annonce::annoncer;
use mc_instance::launch::{Command, Session};
use mc_pack::Partie;

#[cfg(unix)]
fn partie(atelier: &Atelier, cible: Option<&str>, explicite: bool) -> Partie {
    let options = atelier.options();
    let instance = options.layout.instance("samflix");
    instance.create().unwrap();

    Partie {
        instance,
        lock: verrou(vec![entree("jei", "both")]),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), "true".into()],
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "0123456789abcdef"),
        cible: cible.map(str::to_string),
        demande_explicite: explicite,
        environnement: mc_log::Environment::Production,
    }
}

/// La provenance du serveur est dite, pas seulement l'adresse : quelqu'un qui
/// diagnostique une éjection a besoin de savoir s'il regarde le pack ou un
/// `--serveur`.
#[cfg(unix)]
#[test]
fn l_annonce_traverse_ses_trois_provenances() {
    let atelier = Atelier::neuf("annonce");
    atelier.pack_installe(Vec::new());

    annoncer(&partie(&atelier, Some("mc.ggy.info"), false));
    annoncer(&partie(&atelier, Some("essai.invalid"), true));
    annoncer(&partie(&atelier, None, false));
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
