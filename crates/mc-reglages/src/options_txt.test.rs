use super::fusionner;
use crate::types::{Fenetre, Jeu, ModeFenetre};

fn cle(texte: &str, nom: &str) -> Option<String> {
    texte
        .lines()
        .find_map(|ligne| ligne.strip_prefix(&format!("{nom}:")))
        .map(str::to_string)
}

/// LA propriété du module : ce qui ne nous appartient pas reste intact, à sa
/// place et dans son ordre.
///
/// `version:` en particulier — le jeu fonde ses correctifs de données dessus,
/// et la perdre lui ferait traiter le fichier comme venant d'une version
/// inconnue.
#[test]
fn ce_qui_ne_nous_appartient_pas_reste_intact() {
    let avant =
        "version:3955\nlang:fr_fr\nkey_key.attack:key.mouse.left\nsoundCategory_master:0.7\n";

    let apres = fusionner(avant, &Jeu::default(), &Fenetre::default());

    assert_eq!(cle(&apres, "version").as_deref(), Some("3955"));
    assert_eq!(cle(&apres, "lang").as_deref(), Some("fr_fr"));
    assert_eq!(
        cle(&apres, "key_key.attack").as_deref(),
        Some("key.mouse.left")
    );
    assert_eq!(cle(&apres, "soundCategory_master").as_deref(), Some("0.7"));
    // Et dans leur ordre : les quatre lignes d'origine ouvrent toujours le
    // fichier.
    let lignes: Vec<&str> = apres.lines().take(4).collect();
    assert_eq!(lignes[0], "version:3955");
    assert_eq!(lignes[1], "lang:fr_fr");
}

/// Une clé qui existe déjà est remplacée SUR PLACE, pas ajoutée à la fin. Un
/// fichier qui gagnerait un doublon à chaque enregistrement finirait par
/// compter des centaines de lignes, et Minecraft ne lirait que la première.
#[test]
fn une_cle_deja_la_est_remplacee_sur_place_et_une_seule_fois() {
    let avant = "renderDistance:8\nlang:fr_fr\n";
    let jeu = Jeu {
        render_distance: 16,
        ..Jeu::default()
    };

    let apres = fusionner(avant, &jeu, &Fenetre::default());

    assert_eq!(
        apres
            .lines()
            .filter(|l| l.starts_with("renderDistance:"))
            .count(),
        1,
        "{apres}"
    );
    assert_eq!(cle(&apres, "renderDistance").as_deref(), Some("16"));
    // Sur place : elle ouvre toujours le fichier.
    assert!(apres.starts_with("renderDistance:16\n"), "{apres}");
}

/// Une clé absente est ajoutée à la fin.
#[test]
fn une_cle_absente_est_ajoutee() {
    let apres = fusionner("lang:fr_fr\n", &Jeu::default(), &Fenetre::default());
    assert_eq!(
        cle(&apres, "renderDistance").as_deref(),
        Some(&*Jeu::default().render_distance.to_string())
    );
}

/// Un fichier vide donne nos clés, et rien d'autre.
#[test]
fn un_fichier_vide_donne_nos_cles() {
    let apres = fusionner("", &Jeu::default(), &Fenetre::default());
    assert!(cle(&apres, "renderDistance").is_some());
    assert!(cle(&apres, "fullscreen").is_some());
    assert!(apres.ends_with('\n'));
}

/// Une ligne sans deux-points n'est pas une clé : on la garde plutôt que de la
/// deviner. Les fichiers d'anciennes versions en contiennent.
#[test]
fn une_ligne_sans_deux_points_est_conservee() {
    let avant = "une ligne bizarre\nlang:fr_fr\n";
    let apres = fusionner(avant, &Jeu::default(), &Fenetre::default());
    assert!(apres.contains("une ligne bizarre"), "{apres}");
}

/// `fullscreen` suit le MODE de fenêtre, pas un champ séparé. C'est ce qui
/// empêche les deux de diverger quand le joueur appuie sur F11 en partie.
#[test]
fn le_plein_ecran_ecrit_suit_le_mode() {
    for (mode, attendu) in [
        (ModeFenetre::Fenetree, "false"),
        (ModeFenetre::Maximisee, "false"),
        (ModeFenetre::PleinEcran, "true"),
    ] {
        let fenetre = Fenetre {
            mode,
            ..Fenetre::default()
        };
        let apres = fusionner("", &Jeu::default(), &fenetre);
        assert_eq!(
            cle(&apres, "fullscreen").as_deref(),
            Some(attendu),
            "{mode:?}"
        );
    }
}

/// Au-delà de 260, Minecraft n'attend pas un nombre mais le mot « max » :
/// écrire 261 y serait relu comme invalide et le jeu retomberait sur son
/// défaut, sans rien dire. On plafonne donc à 260.
#[test]
fn le_plafond_d_images_ne_depasse_jamais_deux_cent_soixante() {
    let jeu = Jeu {
        max_fps: 260,
        ..Jeu::default()
    };
    let apres = fusionner("", &jeu, &Fenetre::default());
    assert_eq!(cle(&apres, "maxFps").as_deref(), Some("260"));
}

/// `graphicsMode` n'est PAS écrit : les shaders le pilotent, et l'imposer
/// depuis le launcher déferait ce qu'Iris a réglé.
#[test]
fn le_mode_graphique_n_est_jamais_impose() {
    let avant = "graphicsMode:0\n";
    let apres = fusionner(avant, &Jeu::default(), &Fenetre::default());
    assert_eq!(cle(&apres, "graphicsMode").as_deref(), Some("0"));
}

/// Deux fusions d'affilée donnent le même texte : sans cette propriété, le
/// fichier changerait de forme à chaque ouverture de la page de configuration,
/// et un joueur qui versionne son instance verrait un diff sans avoir rien
/// touché.
#[test]
fn fusionner_deux_fois_ne_change_rien_de_plus() {
    let avant = "version:3955\nlang:fr_fr\nrenderDistance:8\n";
    let une = fusionner(avant, &Jeu::default(), &Fenetre::default());
    let deux = fusionner(&une, &Jeu::default(), &Fenetre::default());
    assert_eq!(une, deux);
}
