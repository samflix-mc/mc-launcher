use super::{charger_depuis, chemin, ecrire_protege, effacer_de, enregistrer_dans};

fn dossier(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-auth-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&chemin).ok();
    chemin
}

/// Le fichier porte un jeton de rafraîchissement : un poste partagé ne doit
/// pas le laisser lire par le voisin.
#[cfg(unix)]
#[test]
fn la_session_n_est_lisible_que_par_son_proprietaire() {
    use std::os::unix::fs::PermissionsExt;

    let dir = dossier("droits");
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

#[test]
fn ce_qui_est_ecrit_se_relit_a_l_identique() {
    let dir = dossier("aller-retour");
    let fichier = dir.join("session.json");
    let etat = serde_json::json!({
        "refresh_token": "M.R3_BAY.secret",
        "expires_at": 1_790_000_000u64
    });

    enregistrer_dans(&fichier, &etat).expect("écriture");
    assert_eq!(charger_depuis(&fichier), Some(etat));

    std::fs::remove_dir_all(&dir).ok();
}

/// Le répertoire de configuration peut ne pas exister au premier lancement :
/// l'enregistrement le crée plutôt que d'échouer.
#[test]
fn le_repertoire_de_configuration_est_cree_au_besoin() {
    let dir = dossier("creation");
    let fichier = dir.join("encore").join("plus").join("loin").join("s.json");

    enregistrer_dans(&fichier, &serde_json::json!({})).expect("écriture");
    assert!(fichier.is_file());

    std::fs::remove_dir_all(&dir).ok();
}

/// Un fichier corrompu vaut « pas de session » : refuser de démarrer pour
/// autant empêcherait de jouer, et la seule issue serait d'aller supprimer un
/// fichier à la main.
#[test]
fn une_session_corrompue_vaut_absence_de_session() {
    let dir = dossier("corrompue");
    std::fs::create_dir_all(&dir).unwrap();
    let fichier = dir.join("session.json");
    std::fs::write(&fichier, b"{ceci n'est pas du JSON").unwrap();

    assert!(charger_depuis(&fichier).is_none());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn aucune_session_enregistree_n_est_pas_une_erreur() {
    let dir = dossier("absente");
    assert!(charger_depuis(&dir.join("session.json")).is_none());
}

/// Se déconnecter deux fois de suite doit rester sans effet, pas échouer.
#[test]
fn oublier_une_session_absente_reste_sans_effet() {
    let dir = dossier("oubli");
    let fichier = dir.join("session.json");

    enregistrer_dans(&fichier, &serde_json::json!({"a": 1})).unwrap();
    effacer_de(&fichier).expect("première suppression");
    assert!(!fichier.exists());
    effacer_de(&fichier).expect("seconde suppression");

    std::fs::remove_dir_all(&dir).ok();
}

/// Un répertoire là où l'on attend un fichier : la suppression échoue, et le
/// message doit nommer le chemin fautif.
#[test]
fn une_suppression_impossible_nomme_le_chemin() {
    let dir = dossier("impossible");
    std::fs::create_dir_all(dir.join("session.json")).unwrap();

    let erreur = effacer_de(&dir.join("session.json")).expect_err("c'est un répertoire");
    assert!(format!("{erreur:#}").contains("session.json"), "{erreur:#}");

    std::fs::remove_dir_all(&dir).ok();
}

/// Les trois opérations lisent l'emplacement dans l'environnement. Les
/// éprouver ensemble vérifie qu'elles désignent bien le même fichier : une
/// asymétrie entre l'écriture et la lecture déconnecterait le joueur à chaque
/// lancement, sans rien dire.
///
/// Les enveloppes de `super::super` ne sont pas éprouvées ici : elles passent
/// d'abord par le trousseau du système, et un test qui y écrirait toucherait au
/// portefeuille de la machine qui exécute la suite.
#[test]
fn les_operations_sur_le_fichier_designent_le_meme_emplacement() {
    let dir = dossier("enveloppes");
    std::fs::create_dir_all(&dir).unwrap();

    // SAFETY : la variable est restaurée avant la fin du test, et ce crate ne
    // lance aucun sous-processus.
    let precedent = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &dir);
    }

    let etat = serde_json::json!({"refresh_token": "M.R3_BAY.secret"});
    enregistrer_dans(&chemin(), &etat).expect("écriture");
    assert_eq!(chemin(), dir.join("samflix-mc").join("session.json"));
    assert_eq!(charger_depuis(&chemin()), Some(etat));
    effacer_de(&chemin()).expect("suppression");
    assert!(charger_depuis(&chemin()).is_none());

    unsafe {
        match precedent {
            Some(valeur) => std::env::set_var("XDG_CONFIG_HOME", valeur),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}
