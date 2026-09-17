//! Ce que la barre affiche, et surtout ce qu'elle ne doit jamais afficher.
//!
//! Les fautes visées ici sont celles qui ne font tomber aucune compilation et
//! qu'on ne voit qu'en regardant une installation réelle pendant dix minutes :
//! une barre qui dépasse cent pour cent, un temps restant infini, un compteur
//! qui repasse par zéro.

use super::*;

fn suivi() -> Suivi {
    Suivi::default()
}

#[test]
fn les_octets_recus_s_additionnent() {
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Recus(1_000));
    suivi.telechargement(mc_dl::Avancement::Recus(2_500));

    assert_eq!(suivi.photo().octets, 3_500);
}

#[test]
fn un_reessai_ne_fait_pas_depasser_la_barre() {
    // Une tentative perdue à mi-corps, puis reprise : les octets abandonnés
    // sont retirés, sinon la barre compterait deux fois le même fichier.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Recus(800));
    suivi.telechargement(mc_dl::Avancement::Perdus(800));
    suivi.telechargement(mc_dl::Avancement::Recus(1_000));

    assert_eq!(suivi.photo().octets, 1_000);
}

#[test]
fn retirer_plus_que_compte_ne_repasse_pas_par_zero() {
    // Sur un entier non signé, la soustraction qui déborde donne seize
    // exaoctets. C'est le genre de chiffre qui arrive à l'écran.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Recus(100));
    suivi.telechargement(mc_dl::Avancement::Perdus(5_000));

    assert_eq!(suivi.photo().octets, 0);
}

#[test]
fn un_fichier_deja_present_fait_avancer_la_barre_sans_rien_telecharger() {
    // Le cas d'une réinstallation : sans cela, la barre resterait à zéro de
    // bout en bout alors que tout est déjà sur le disque.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Fini {
        fichier: "jei.jar",
        etat: mc_dl::Fetched::AlreadyPresent,
        octets: 4_000,
    });

    let photo = suivi.photo();
    assert_eq!(photo.octets, 4_000);
    assert_eq!(photo.fichiers, 1);
}

#[test]
fn un_fichier_telecharge_n_est_pas_compte_deux_fois() {
    // Ses octets sont déjà passés par `Recus` : les ajouter à nouveau à
    // l'arrivée ferait terminer la barre au double du total.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Recus(4_000));
    suivi.telechargement(mc_dl::Avancement::Fini {
        fichier: "jei.jar",
        etat: mc_dl::Fetched::Downloaded,
        octets: 4_000,
    });

    let photo = suivi.photo();
    assert_eq!(photo.octets, 4_000);
    assert_eq!(photo.fichiers, 1);
}

#[test]
fn un_nouveau_lot_repart_de_zero() {
    // Les étapes s'enchaînent — bibliothèques, puis assets, puis mods — et
    // chacune annonce son propre total. Cumuler ferait une barre qui n'avance
    // jamais et un total qui ne veut rien dire.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Lot {
        fichiers: 10,
        octets: 1_000,
    });
    suivi.telechargement(mc_dl::Avancement::Recus(1_000));
    suivi.telechargement(mc_dl::Avancement::Lot {
        fichiers: 2_500,
        octets: 830_000_000,
    });

    let photo = suivi.photo();
    assert_eq!(photo.octets, 0);
    assert_eq!(photo.total, 830_000_000);
    assert_eq!(photo.fichiers, 0);
    assert_eq!(photo.fichiers_total, 2_500);
}

#[test]
fn le_fichier_en_cours_est_celui_qui_vient_de_commencer() {
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Debut {
        fichier: "lwjgl.jar",
        octets: Some(900),
    });
    suivi.telechargement(mc_dl::Avancement::Debut {
        fichier: "jei-1.21.1.jar",
        octets: None,
    });

    assert_eq!(suivi.photo().fichier.as_deref(), Some("jei-1.21.1.jar"));
}

#[test]
fn la_phase_et_la_note_traversent_telles_quelles() {
    let suivi = suivi();
    suivi.phase(Phase::Mods);
    suivi.note("128 mods, dont 43 ajoutés par résolution");

    let photo = suivi.photo();
    assert_eq!(photo.phase, Phase::Mods);
    assert_eq!(
        photo.note.as_deref(),
        Some("128 mods, dont 43 ajoutés par résolution")
    );
}

#[test]
fn le_debit_instantane_se_calcule_sur_l_intervalle() {
    assert_eq!(debit_instantane(1_000_000, 2.0), 500_000.0);
}

#[test]
fn un_intervalle_nul_ne_donne_pas_l_infini() {
    // Deux photos dans la même microseconde : diviser par zéro afficherait
    // « inf o/s », puis une conversion en entier au comportement indéfini.
    assert_eq!(debit_instantane(1_000, 0.0), 0.0);
    assert_eq!(debit_instantane(1_000, -1.0), 0.0);
}

#[test]
fn le_lissage_tend_vers_la_mesure_sans_la_rejoindre_d_un_coup() {
    // La valeur doit bouger — un lissage à zéro figerait l'affichage — sans
    // atteindre la mesure immédiatement, sinon il ne lisse rien.
    let apres = lisser(0.0, 1_000.0);
    assert!(apres > 0.0 && apres < 1_000.0, "{apres}");

    // Et en répétant, elle converge.
    let mut valeur = 0.0;
    for _ in 0..100 {
        valeur = lisser(valeur, 1_000.0);
    }
    assert!((valeur - 1_000.0).abs() < 1.0, "{valeur}");
}

#[test]
fn le_temps_restant_se_deduit_du_debit() {
    // 500 Mo restants à 10 Mo/s : cinquante secondes.
    assert_eq!(restant(1_000, 500, 10.0), Some(50));
}

#[test]
fn le_temps_restant_s_arrondit_vers_le_haut() {
    // Tronquer afficherait « 0 s » pendant la dernière seconde.
    assert_eq!(restant(1_000, 999, 2.0), Some(1));
}

#[test]
fn sans_total_annonce_aucun_temps_n_est_promis() {
    // La source ne publie pas les tailles : mieux vaut ne rien afficher qu'une
    // estimation inventée.
    assert_eq!(restant(0, 500, 10.0), None);
}

#[test]
fn sans_debit_aucun_temps_n_est_promis() {
    // Diviser par zéro donnerait l'infini, puis un entier quelconque.
    assert_eq!(restant(1_000, 500, 0.0), None);
    assert_eq!(restant(1_000, 500, -1.0), None);
}

#[test]
fn un_total_depasse_ne_promet_rien_non_plus() {
    // Les mods de CurseForge sans clé comptent pour zéro dans le total : on
    // dépasse alors ce qui était annoncé. Afficher « 0 s » laisserait croire
    // que c'est fini alors qu'il reste des fichiers.
    assert_eq!(restant(1_000, 1_000, 10.0), None);
    assert_eq!(restant(1_000, 5_000, 10.0), None);
}

/// Le défaut observé en installation réelle : la barre restait pleine pendant
/// que la résolution des mods travaillait encore. Le lot précédent était
/// complet, le suivant pas encore annoncé — et cent pour cent, dans cet
/// intervalle, est un mensonge.
#[test]
fn un_lot_complet_ne_compte_plus_comme_un_telechargement_en_cours() {
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Lot {
        fichiers: 2,
        octets: 200,
    });
    assert!(suivi.photo().actif, "le lot vient de commencer");

    for _ in 0..2 {
        suivi.telechargement(mc_dl::Avancement::Fini {
            fichier: "jei.jar",
            etat: mc_dl::Fetched::Downloaded,
            octets: 100,
        });
    }

    assert!(!suivi.photo().actif, "tous les fichiers sont réglés");
}

#[test]
fn sans_lot_annonce_rien_n_est_en_cours() {
    // L'installateur NeoForge tourne dans sa JVM sans rien télécharger : une
    // barre chiffrée n'aurait aucun sens.
    assert!(!en_cours(0, 0));
    assert!(en_cours(0, 10));
    assert!(!en_cours(10, 10));
    assert!(!en_cours(11, 10));
}

/// « Prêt à jouer » n'est pas une étape qu'on exécute, c'est le résultat des
/// neuf précédentes. L'éclairer comme une étape en cours laisserait croire
/// qu'il reste à attendre.
#[test]
fn une_phase_achevee_se_distingue_d_une_phase_qui_travaille() {
    let suivi = suivi();

    suivi.phase(Phase::Mods);
    assert!(!suivi.photo().achevee);

    suivi.termine(Phase::Pret);
    let photo = suivi.photo();
    assert_eq!(photo.phase, Phase::Pret);
    assert!(photo.achevee);
}

/// Sans cela, la barre du dernier téléchargement resterait affichée pleine
/// sous « Prêt à jouer », comme si quelque chose continuait.
#[test]
fn terminer_efface_le_lot_et_le_fichier_en_cours() {
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Lot {
        fichiers: 3,
        octets: 300,
    });
    suivi.telechargement(mc_dl::Avancement::Debut {
        fichier: "jei.jar",
        octets: Some(100),
    });
    suivi.telechargement(mc_dl::Avancement::Recus(300));

    suivi.termine(Phase::Pret);

    let photo = suivi.photo();
    assert_eq!(photo.total, 0);
    assert_eq!(photo.octets, 0);
    assert_eq!(photo.fichiers_total, 0);
    assert_eq!(photo.fichier, None);
    assert!(!photo.actif);
}

/// Reprendre une installation après l'avoir terminée doit rallumer l'état
/// « en cours » : sinon le chemin resterait figé sur « Prêt ».
#[test]
fn reprendre_une_phase_annule_l_achevement() {
    let suivi = suivi();
    suivi.termine(Phase::Pret);

    suivi.phase(Phase::Minecraft);

    assert!(!suivi.photo().achevee);
}

#[test]
fn la_photo_se_serialise_en_camel_case() {
    // Les noms de champs sont ce que lit le TypeScript.
    let suivi = suivi();
    suivi.telechargement(mc_dl::Avancement::Lot {
        fichiers: 3,
        octets: 99,
    });

    let json = serde_json::to_value(suivi.photo()).expect("sérialisation");
    assert_eq!(json["fichiersTotal"], 3);
    assert_eq!(json["total"], 99);
    assert_eq!(json["phase"], "connexion");
}
