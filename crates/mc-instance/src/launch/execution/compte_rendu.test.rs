use super::Outcome;

#[cfg(unix)]
fn statut(brut: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(brut)
}

#[cfg(unix)]
#[test]
fn une_sortie_normale_n_ouvre_pas_d_incident() {
    // Code 0 : écran de fin, ou fenêtre fermée.
    let compte_rendu = Outcome::from_status(&statut(0));
    assert_eq!(compte_rendu, Outcome::Normal);
    assert!(!compte_rendu.is_failure());
}

/// Fermer le jeu par Ctrl+C n'est pas une panne. Le confondre avec une erreur
/// remplissait le tableau de bord d'incidents à chaque fermeture.
#[cfg(unix)]
#[test]
fn un_arret_demande_de_l_exterieur_n_est_pas_une_panne() {
    // Statut brut Unix : l'octet bas porte le signal. 15 = SIGTERM.
    let compte_rendu = Outcome::from_status(&statut(15));
    assert_eq!(compte_rendu, Outcome::Interrupted { signal: 15 });
    assert!(!compte_rendu.is_failure());
}

/// Un shell traduit un signal en 128 + n. `status.signal()` ne le voit pas
/// quand le code traverse un intermédiaire : 143 est un SIGTERM, 130 un Ctrl+C.
#[cfg(unix)]
#[test]
fn un_signal_traduit_par_un_shell_est_reconnu_aussi() {
    for (code, signal) in [(143, 15), (130, 2)] {
        let compte_rendu = Outcome::from_status(&statut(code << 8));
        assert_eq!(
            compte_rendu,
            Outcome::Interrupted { signal },
            "pour le code {code}"
        );
        assert!(!compte_rendu.is_failure());
    }
}

#[cfg(unix)]
#[test]
fn un_arret_sur_erreur_ouvre_un_incident() {
    let compte_rendu = Outcome::from_status(&statut(1 << 8));
    assert_eq!(compte_rendu, Outcome::Failed { code: 1 });
    assert!(compte_rendu.is_failure());
}

/// La borne haute compte : au-delà de 192, un code n'est plus un signal
/// traduit mais un code de sortie que le jeu a choisi.
#[cfg(unix)]
#[test]
fn un_code_hors_de_la_plage_des_signaux_reste_une_erreur() {
    assert_eq!(
        Outcome::from_status(&statut(200 << 8)),
        Outcome::Failed { code: 200 }
    );
    assert_eq!(
        Outcome::from_status(&statut(128 << 8)),
        Outcome::Failed { code: 128 }
    );
}

/// Un arrêt dont le système ne rend aucun code ne peut venir d'aucun
/// programme : le repli le dit par une valeur qu'aucun processus ne produit.
/// Rendre 1 à sa place ferait passer un arrêt inexpliqué pour une erreur
/// ordinaire du jeu, et le rapport d'incident chercherait une exception qui
/// n'existe pas.
#[test]
fn un_arret_sans_code_ne_se_confond_pas_avec_une_erreur_du_jeu() {
    assert_eq!(Outcome::echec(None), Outcome::Failed { code: -1 });
    assert_eq!(Outcome::echec(Some(1)), Outcome::Failed { code: 1 });
}
