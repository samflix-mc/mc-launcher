use super::{Avant, EtatLocal, SCHEMA, decider, purger};
use crate::essais::Atelier;

fn etat(generation: u32) -> EtatLocal {
    EtatLocal::neuf("aa".into(), generation, "2026-09-18T00:00:00Z".into())
}

// --- La décision, qui est une fonction pure --------------------------------

/// Sans état, on ne sait rien de ce qui est posé : purger.
///
/// C'est le cas de TOUT poste déjà installé au premier lancement d'après cette
/// version — cent pour cent de la population, une fois. Rien ne l'annonce
/// aujourd'hui, et c'est à noter dans les notes de version.
#[test]
fn sans_etat_on_purge() {
    assert_eq!(decider(None, 0), Avant::Purger);
    assert_eq!(decider(None, 7), Avant::Purger);
}

/// Même génération : rien de spécial, différentiel. C'est le cas courant, et
/// celui qui doit rester bon marché — un demi-gigaoctet ne se retélécharge pas
/// à chaque partie.
#[test]
fn a_generation_egale_on_installe_par_difference() {
    assert_eq!(decider(Some(&etat(0)), 0), Avant::Differentiel);
    assert_eq!(decider(Some(&etat(3)), 3), Avant::Differentiel);
}

/// Génération demandée supérieure : celui qui publie a demandé une
/// réinstallation propre.
#[test]
fn une_generation_superieure_declenche_la_purge() {
    assert_eq!(decider(Some(&etat(0)), 1), Avant::Purger);
    assert_eq!(decider(Some(&etat(3)), 4), Avant::Purger);
    // Un saut de plusieurs générations — un joueur qui n'a pas lancé le
    // launcher depuis des mois — purge une seule fois, pas une par génération.
    assert_eq!(decider(Some(&etat(1)), 9), Avant::Purger);
}

/// Génération demandée INFÉRIEURE : on ne purge pas.
///
/// C'est le `<` plutôt que le `!=`, et c'est le test qui le tient. Revenir à
/// une génération antérieure veut dire qu'on republie un état qu'on savait
/// bon ; effacer l'installation à cette occasion punirait le joueur d'une
/// décision d'exploitation. Le différentiel remettra les fichiers d'avant.
#[test]
fn un_retour_en_arriere_ne_purge_pas() {
    assert_eq!(decider(Some(&etat(5)), 4), Avant::Differentiel);
    assert_eq!(decider(Some(&etat(5)), 0), Avant::Differentiel);
}

// --- L'état sur le disque --------------------------------------------------

#[test]
fn un_etat_ecrit_se_relit_a_l_identique() {
    let atelier = Atelier::neuf("etat-aller-retour");
    let chemin = atelier.racine.join("instances").join("x").join("etat.json");

    let ecrit = etat(2);
    ecrit.ecrire(&chemin).expect("écriture");

    assert_eq!(EtatLocal::lire(&chemin), Some(ecrit));
}

/// Un état absent n'est pas une erreur : c'est « on ne sait pas », et la
/// conduite qui suit est la même que pour un état illisible.
#[test]
fn un_etat_absent_rend_none() {
    let atelier = Atelier::neuf("etat-absent");
    assert_eq!(
        EtatLocal::lire(&atelier.racine.join("nulle-part.json")),
        None
    );
}

/// Un état illisible vaut une absence, et ne fait pas échouer le lancement.
/// Le contraire empêcherait de jouer à cause d'un fichier corrompu — un
/// arrêt brutal pendant l'écriture, un disque plein.
#[test]
fn un_etat_casse_rend_none() {
    let atelier = Atelier::neuf("etat-casse");
    let chemin = atelier.ecrire("etat.json", b"{ceci n'est pas du JSON");
    assert_eq!(EtatLocal::lire(&chemin), None);
}

/// Un schéma inconnu vaut une absence, DONC une purge.
///
/// C'est le contrôle que le verrou n'a jamais eu : son champ `schema` est
/// écrit et jamais relu. Reconduire ce champ mort dans un fichier neuf aurait
/// été faire exprès l'erreur qu'on reproche.
#[test]
fn un_schema_inconnu_vaut_une_absence_donc_une_purge() {
    let atelier = Atelier::neuf("etat-schema");
    let futur = format!(
        r#"{{"schema":{},"verrou_sha512":"aa","generation":3,"pose_le":"2026-09-18T00:00:00Z"}}"#,
        SCHEMA + 1
    );
    let chemin = atelier.ecrire("etat.json", futur.as_bytes());

    let relu = EtatLocal::lire(&chemin);
    assert_eq!(relu, None);
    assert_eq!(decider(relu.as_ref(), 3), Avant::Purger);
}

// --- La purge --------------------------------------------------------------

/// Ce que la purge efface, et surtout ce qu'elle N'efface PAS.
///
/// Le test le plus important du module : perdre un monde pour rattraper un
/// renommage de mod serait un remède pire que le mal.
#[test]
fn la_purge_n_efface_que_ce_que_le_launcher_a_pose() {
    let atelier = Atelier::neuf("purge");
    let jeu = atelier.racine.join("minecraft");

    // Ce que le launcher pose, et qu'il a le droit d'effacer.
    for efface in ["mods", "shaderpacks", "resourcepacks", "libraries"] {
        std::fs::create_dir_all(jeu.join(efface)).unwrap();
        std::fs::write(jeu.join(efface).join("un.jar"), b"x").unwrap();
    }
    // Un sous-répertoire et un .jar.disabled : le résidu exact que le
    // différentiel ne voit pas, puisque le verrou ne le décrit plus.
    std::fs::create_dir_all(jeu.join("mods").join("vieux")).unwrap();
    std::fs::write(jeu.join("mods").join("ancien.jar.disabled"), b"x").unwrap();

    // Ce qui appartient au joueur, et que rien n'autorise à toucher.
    for garde in ["saves", "config", "screenshots", "logs", "schematics"] {
        std::fs::create_dir_all(jeu.join(garde)).unwrap();
        std::fs::write(jeu.join(garde).join("precieux"), b"x").unwrap();
    }
    std::fs::write(jeu.join("options.txt"), b"fullscreen:true\n").unwrap();

    let purge = purger(&jeu);

    for efface in ["mods", "shaderpacks", "resourcepacks", "libraries"] {
        assert!(!jeu.join(efface).exists(), "{efface} aurait dû être effacé");
    }
    for garde in ["saves", "config", "screenshots", "logs", "schematics"] {
        assert!(
            jeu.join(garde).join("precieux").exists(),
            "{garde} a été effacé : une soirée de reconfiguration, ou un monde"
        );
    }
    assert!(jeu.join("options.txt").exists());

    assert!(purge.a_eu_lieu());
    assert_eq!(purge.vides.len(), 4);
    assert!(purge.echecs.is_empty(), "{:?}", purge.echecs);
}

/// `config/` n'est PAS effacé, et cela mérite son propre test : c'est le
/// répertoire où le joueur règle ses mods, et le seul dont la suppression se
/// discuterait. Un changement de forme qui l'exigerait demande une décision
/// humaine annoncée, pas un numéro incrémenté dans un fichier.
#[test]
fn la_purge_epargne_la_configuration_des_mods() {
    let atelier = Atelier::neuf("purge-config");
    let jeu = atelier.racine.join("minecraft");
    std::fs::create_dir_all(jeu.join("config").join("jei")).unwrap();
    std::fs::write(jeu.join("config").join("jei").join("jei.ini"), b"x").unwrap();

    purger(&jeu);

    assert!(jeu.join("config").join("jei").join("jei.ini").exists());
}

/// Une purge sur une instance qui n'a rien ne fait rien, et ne se signale pas.
/// C'est le cas du tout premier lancement : annoncer une purge à quelqu'un qui
/// installe pour la première fois ferait croire qu'on lui a effacé quelque
/// chose.
#[test]
fn une_purge_sans_rien_a_effacer_ne_dit_rien() {
    let atelier = Atelier::neuf("purge-vide");
    let jeu = atelier.racine.join("minecraft");
    std::fs::create_dir_all(&jeu).unwrap();

    let purge = purger(&jeu);

    assert!(!purge.a_eu_lieu());
    assert!(purge.vides.is_empty());
}

/// Et une purge sur un répertoire qui n'existe pas du tout ne panique pas.
#[test]
fn une_purge_sur_rien_du_tout_ne_panique_pas() {
    let atelier = Atelier::neuf("purge-neant");
    let purge = purger(&atelier.racine.join("jamais-cree"));
    assert!(!purge.a_eu_lieu());
}
