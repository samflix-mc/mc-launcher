use super::*;

/// Le fichier porte un jeton de rafraîchissement : un poste partagé ne doit
/// pas le laisser lire par le voisin.
#[cfg(unix)]
#[test]
fn la_session_n_est_lisible_que_par_son_proprietaire() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("mc-auth-droits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("répertoire de test");
    let fichier = dir.join("session.json");

    ecrire_protege(&fichier, b"{}").expect("écriture");
    let mode = std::fs::metadata(&fichier)
        .expect("fichier écrit")
        .permissions()
        .mode();

    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(mode & 0o777, 0o600, "mode {:o}", mode & 0o777);
}

/// Le secret vit dans la configuration, pas dans le cache : il ne se
/// reconstruit pas, et un nettoyage des données ne doit pas déconnecter.
#[test]
fn la_session_vit_a_cote_de_la_configuration() {
    let chemin = chemin();
    assert!(chemin.ends_with("samflix-mc/session.json"), "{chemin:?}");
}
