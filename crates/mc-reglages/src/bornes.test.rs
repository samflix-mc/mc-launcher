use super::{ECHELLE, FPS, LARGEUR, MEMOIRE, RENDU, SIMULATION, VOILE_PLANCHER};
use crate::types::{Apparence, Fenetre, Jeu, Lanceur, ModeFenetre, Reglages};

/// Un test par borne, ET un à la borne ± 1.
///
/// C'est ce que les conventions de test du dépôt exigent des comparaisons, et
/// ce sans quoi un mutant qui remplace `<` par `<=` survit indéfiniment : à la
/// borne exacte, les deux versions rendent la même chose.
#[test]
fn la_distance_de_rendu_est_bornee_aux_deux_bouts() {
    let cas = [
        (0u8, RENDU.0),             // très en dessous
        (RENDU.0 - 1, RENDU.0),     // juste en dessous
        (RENDU.0, RENDU.0),         // à la borne : inchangé
        (RENDU.0 + 1, RENDU.0 + 1), // juste au-dessus : inchangé
        (RENDU.1 - 1, RENDU.1 - 1),
        (RENDU.1, RENDU.1),
        (255, RENDU.1), // un joueur qui cherche à voir loin
    ];
    for (donne, attendu) in cas {
        let mut jeu = Jeu {
            render_distance: donne,
            ..Jeu::default()
        };
        jeu.valider();
        assert_eq!(jeu.render_distance, attendu, "rendu {donne}");
    }
}

#[test]
fn la_distance_de_simulation_est_bornee_aux_deux_bouts() {
    for (donne, attendu) in [
        (0u8, SIMULATION.0),
        (SIMULATION.0 - 1, SIMULATION.0),
        (SIMULATION.0, SIMULATION.0),
        (SIMULATION.0 + 1, SIMULATION.0 + 1),
        (SIMULATION.1, SIMULATION.1),
        (200, SIMULATION.1),
    ] {
        let mut jeu = Jeu {
            simulation_distance: donne,
            ..Jeu::default()
        };
        jeu.valider();
        assert_eq!(jeu.simulation_distance, attendu, "simulation {donne}");
    }
}

/// Au-delà de 260, Minecraft écrit `max` et ne limite plus du tout : un joueur
/// qui pose 9999 croirait avoir demandé une limite haute et n'en aurait aucune.
#[test]
fn le_plafond_d_images_est_borne_aux_deux_bouts() {
    for (donne, attendu) in [
        (0u16, FPS.0),
        (FPS.0 - 1, FPS.0),
        (FPS.0, FPS.0),
        (FPS.0 + 1, FPS.0 + 1),
        (FPS.1, FPS.1),
        (FPS.1 + 1, FPS.1),
        (9999, FPS.1),
    ] {
        let mut jeu = Jeu {
            max_fps: donne,
            ..Jeu::default()
        };
        jeu.valider();
        assert_eq!(jeu.max_fps, attendu, "fps {donne}");
    }
}

/// Zéro est une VALEUR et non une absence : c'est « échelle automatique ». La
/// borner à 1 retirerait au joueur le réglage par défaut du jeu.
#[test]
fn l_echelle_zero_est_une_valeur_et_non_une_absence() {
    let mut jeu = Jeu {
        gui_scale: 0,
        ..Jeu::default()
    };
    jeu.valider();
    assert_eq!(jeu.gui_scale, 0);

    let mut trop = Jeu {
        gui_scale: 9,
        ..Jeu::default()
    };
    trop.valider();
    assert_eq!(trop.gui_scale, ECHELLE.1);
}

/// `None` n'est pas une valeur hors bornes : c'est le choix « laisse la JVM
/// décider ». La remplacer par le plancher retirerait au joueur une option.
#[test]
fn une_memoire_absente_reste_absente() {
    let mut lanceur = Lanceur {
        memoire_mo: None,
        ..Lanceur::default()
    };
    lanceur.valider();
    assert_eq!(lanceur.memoire_mo, None);
}

#[test]
fn la_memoire_est_bornee_aux_deux_bouts() {
    for (donne, attendu) in [
        (0u32, MEMOIRE.0),
        (MEMOIRE.0 - 1, MEMOIRE.0),
        (MEMOIRE.0, MEMOIRE.0),
        (MEMOIRE.0 + 1, MEMOIRE.0 + 1),
        (MEMOIRE.1, MEMOIRE.1),
        (MEMOIRE.1 + 1, MEMOIRE.1),
    ] {
        let mut lanceur = Lanceur {
            memoire_mo: Some(donne),
            ..Lanceur::default()
        };
        lanceur.valider();
        assert_eq!(lanceur.memoire_mo, Some(attendu), "mémoire {donne}");
    }
}

#[test]
fn la_taille_de_fenetre_est_bornee() {
    let mut fenetre = Fenetre {
        largeur: 1,
        hauteur: 1,
        ..Fenetre::default()
    };
    fenetre.valider();
    assert_eq!(fenetre.largeur, LARGEUR.0);

    let mut enorme = Fenetre {
        largeur: u32::MAX,
        hauteur: u32::MAX,
        ..Fenetre::default()
    };
    enorme.valider();
    assert_eq!(enorme.largeur, LARGEUR.1);
}

// --- Le voile, et c'est la borne qui compte --------------------------------

/// Le voile est borné PAR LE BAS, et ce n'est pas un confort : en dessous, le
/// texte cesse de tenir le contraste sur l'image la plus claire, et les
/// libellés deviennent illisibles sur une partie de l'écran seulement — ce qui
/// ressemble à un défaut de rendu et non à un réglage.
#[test]
fn le_voile_ne_descend_pas_sous_le_plancher() {
    for donne in [0.0, 0.1, VOILE_PLANCHER - 0.01] {
        let mut apparence = Apparence {
            voile: donne,
            ..Apparence::default()
        };
        apparence.valider();
        assert_eq!(apparence.voile, VOILE_PLANCHER, "voile {donne}");
    }
}

/// À la borne et juste au-dessus, rien ne bouge.
#[test]
fn le_voile_au_plancher_et_au_dessus_ne_bouge_pas() {
    for donne in [VOILE_PLANCHER, VOILE_PLANCHER + 0.01, 0.9, 1.0] {
        let mut apparence = Apparence {
            voile: donne,
            ..Apparence::default()
        };
        apparence.valider();
        assert_eq!(apparence.voile, donne, "voile {donne}");
    }
}

#[test]
fn le_voile_ne_depasse_pas_un() {
    let mut apparence = Apparence {
        voile: 3.0,
        ..Apparence::default()
    };
    apparence.valider();
    assert_eq!(apparence.voile, 1.0);
}

/// Un NaN traverse une comparaison ordinaire sans être vu : `NaN < x` et
/// `NaN > x` sont faux tous les deux. Sans le contrôle explicite, il
/// ressortirait inchangé et rendrait un voile transparent — exactement ce que
/// le plancher existe pour empêcher. Un JSON édité à la main peut en produire.
#[test]
fn un_voile_qui_n_est_pas_un_nombre_retombe_sur_le_defaut() {
    for aberrant in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut apparence = Apparence {
            voile: aberrant,
            ..Apparence::default()
        };
        apparence.valider();
        assert_eq!(apparence.voile, Apparence::default().voile);
    }
}

// --- L'ensemble ------------------------------------------------------------

/// Valider remet le schéma à celui d'aujourd'hui : un fichier relu d'une
/// version antérieure est réécrit à la version courante.
#[test]
fn valider_remet_le_schema_a_jour() {
    let mut reglages = Reglages {
        schema: 0,
        ..Reglages::default()
    };
    reglages.valider();
    assert_eq!(reglages.schema, crate::types::SCHEMA);
}

/// Les défauts sont déjà valides : valider ne doit rien changer à une
/// configuration neuve, sans quoi le premier enregistrement écrirait autre
/// chose que ce que l'écran montrait.
#[test]
fn les_defauts_sont_deja_valides() {
    let mut reglages = Reglages::default();
    let avant = reglages.clone();
    reglages.valider();
    assert_eq!(reglages, avant);
}

/// La clé `fullscreen` d'`options.txt` est DÉRIVÉE du mode, et non un champ à
/// part. C'est ce qui empêche les deux de diverger — et ils divergeraient : F11
/// bascule cette clé en cours de partie et le jeu la persiste, donc un champ
/// indépendant aurait fait du plein écran un interrupteur à sens unique.
#[test]
fn le_plein_ecran_se_deduit_du_mode() {
    for (mode, attendu) in [
        (ModeFenetre::Fenetree, false),
        (ModeFenetre::Maximisee, false),
        (ModeFenetre::PleinEcran, true),
    ] {
        let fenetre = Fenetre {
            mode,
            ..Fenetre::default()
        };
        assert_eq!(fenetre.plein_ecran(), attendu, "{mode:?}");
    }
}
