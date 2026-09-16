use super::Guard;
use super::journal::current_log_name;

#[test]
fn sans_journal_il_n_y_a_pas_de_chemin_a_annoncer() {
    // Le cas d'un répertoire en lecture seule : on perd le fichier, et il ne
    // faut surtout pas en annoncer un qui n'existe pas.
    assert!(Guard::new(None, None, None).log_path().is_none());
}

/// Le chemin est recalculé à chaque appel, jamais figé au démarrage :
/// `rolling::daily` change de fichier à minuit UTC, et une partie commencée
/// avant continue dans le suivant. Un chemin figé désignerait alors un fichier
/// qui existe mais s'arrête avant la panne.
#[test]
fn le_chemin_annonce_porte_la_date_du_jour() {
    let dir = std::path::PathBuf::from("/tmp/mc-log-essai");
    let guard = Guard::new(None, None, Some((dir.clone(), "mc-pack".into())));

    let chemin = guard.log_path().expect("un journal est déclaré");
    assert_eq!(chemin, dir.join(current_log_name("mc-pack")));
    assert!(
        chemin.to_string_lossy().contains("mc-pack.log."),
        "{}",
        chemin.display()
    );
}
