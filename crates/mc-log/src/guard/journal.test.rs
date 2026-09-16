use super::{current_log_name, log_dir};

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
    let dir = std::env::temp_dir().join(format!(
        "mc-log-appender-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let _appender = tracing_appender::rolling::daily(&dir, "mc-pack.log");
    let annonce = dir.join(current_log_name("mc-pack"));

    let existe = annonce.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(existe, "{} n'existe pas", annonce.display());
}
