use super::{report_unresolved, verify};
use crate::commandes::essais::{Atelier, entree, verrou};
use std::process::ExitCode;

/// Une installation conforme rend le succès ; l'appelant s'en sert comme code
/// de sortie, et c'est lui qui porte l'information en CI.
#[test]
fn une_installation_conforme_rend_le_succes() {
    let atelier = Atelier::neuf("verify-ok");
    let source = atelier.pack_installe(vec![entree("jei", "both")]);

    let code = verify(&source, &atelier.options(), false).unwrap();
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

/// Une anomalie de vérification n'est pas une panne du programme : elle décrit
/// l'installation, et c'est le code de sortie qui la porte.
#[test]
fn une_installation_incomplete_rend_l_echec_sans_paniquer() {
    let atelier = Atelier::neuf("verify-ko");
    let source = atelier.pack_installe(vec![entree("jei", "both")]);
    let instance = atelier.options().layout.instance("samflix");
    std::fs::remove_file(instance.mods_dir().join("jei.jar")).unwrap();

    let code = verify(&source, &atelier.options(), false).unwrap();
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
}

/// Une dépendance introuvable n'empêche pas d'installer mais empêchera le jeu
/// de démarrer : elle est signalée là où on la verra.
#[test]
fn les_dependances_introuvables_sont_annoncees() {
    let mut lock = verrou(vec![entree("jei", "both")]);
    lock.unresolved.push(mc_pack::lockfile::LockedMissing {
        mod_id: "bookshelf".into(),
        required_by: "jei".into(),
        side: "both".into(),
    });

    // Elle part sur la sortie d'erreur ; ce que le test retient est que la
    // liste se parcourt sans paniquer, y compris quand elle est vide.
    report_unresolved(&lock);
    report_unresolved(&verrou(Vec::new()));
}
