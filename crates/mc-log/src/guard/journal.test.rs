use super::{current_log_name, log_dir, purge_old_logs, retention};

/// Deux semaines, et le dire en secondes : c'est la seule unité que connaisse
/// `Duration`, et celle où l'erreur ne se voit pas.
#[test]
fn les_journaux_se_gardent_deux_semaines() {
    assert_eq!(retention(), std::time::Duration::from_secs(14 * 86_400));
}

/// Un répertoire de travail propre à ce test.
fn dossier(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-log-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).unwrap();
    chemin
}

/// Recule la date de dernière écriture d'un fichier.
fn vieillir(chemin: &std::path::Path, jours: u64) {
    let quand = std::time::SystemTime::now() - std::time::Duration::from_secs(jours * 24 * 3600);
    let fichier = std::fs::OpenOptions::new()
        .write(true)
        .open(chemin)
        .unwrap();
    fichier
        .set_times(std::fs::FileTimes::new().set_modified(quand))
        .unwrap();
}

#[test]
fn les_journaux_vivent_sous_le_repertoire_de_donnees() {
    assert!(log_dir().ends_with("logs"));
    assert!(log_dir().starts_with(mc_dl::data_dir()));
}

#[test]
fn le_chemin_annonce_est_celui_que_l_appender_ouvre() {
    // C'est le fichier qu'on demande au joueur de joindre : le nommer sans
    // sa date l'envoyait vers un fichier absent. Le nom est calculé de notre
    // côté, donc c'est l'appender lui-même qui doit l'attester — si
    // tracing-appender change de format, ce test tombe au lieu que le
    // launcher se remette silencieusement à désigner un fichier fantôme.
    let dir = dossier("appender");

    let _appender = tracing_appender::rolling::daily(&dir, "mc-pack.log");
    let annonce = dir.join(current_log_name("mc-pack"));

    let existe = annonce.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(existe, "{} n'existe pas", annonce.display());
}

/// Quatorze jours : assez pour qu'un joueur retrouve la trace d'un incident de
/// la semaine, pas assez pour que le répertoire grossisse indéfiniment.
#[test]
fn les_journaux_trop_vieux_disparaissent_et_les_autres_restent() {
    let dir = dossier("purge");

    let vieux = dir.join("mc-pack.log.2020-01-01");
    let recent = dir.join("mc-pack.log.2026-09-17");
    std::fs::write(&vieux, b"vieux").unwrap();
    std::fs::write(&recent, b"recent").unwrap();
    vieillir(&vieux, 30);

    purge_old_logs(&dir);

    let vieux_existe = vieux.exists();
    let recent_existe = recent.exists();
    std::fs::remove_dir_all(&dir).ok();

    assert!(!vieux_existe, "un journal de trente jours a survécu");
    assert!(recent_existe, "un journal du jour a été supprimé");
}

/// La purge ne touche qu'aux journaux. Le répertoire peut contenir autre
/// chose — une capture, une note laissée par un joueur —, et l'effacer serait
/// une surprise désagréable.
#[test]
fn la_purge_laisse_ce_qui_n_est_pas_un_journal() {
    let dir = dossier("purge-autre");

    let etranger = dir.join("capture.png");
    std::fs::write(&etranger, b"png").unwrap();
    vieillir(&etranger, 30);

    purge_old_logs(&dir);

    let survit = etranger.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(survit, "un fichier étranger a été supprimé");
}

/// Un répertoire absent n'est pas une faute : c'est le cas au tout premier
/// lancement, juste avant que la couche fichier ne le crée.
#[test]
fn purger_un_repertoire_absent_ne_fait_rien() {
    purge_old_logs(&std::env::temp_dir().join("mc-log-repertoire-qui-n-existe-pas"));
}
