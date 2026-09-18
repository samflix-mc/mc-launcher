use super::{Deroulement, doit_rattraper};
use crate::comparaison::{Action, Ecart, EtatDuPack};

fn etat(ecart: Ecart, hors_ligne: bool) -> EtatDuPack {
    EtatDuPack {
        action: Action::Jouer,
        ecart,
        hors_ligne,
        installe: true,
        nom: Some("samflix".into()),
        version: None,
        java: Some(21),
        mods: 0,
        generation: 0,
    }
}

/// Les trois écarts qui demandent d'agir.
#[test]
fn ce_qui_a_bouge_se_rattrape() {
    for ecart in [Ecart::Absent, Ecart::MiseAJour, Ecart::Reinstallation] {
        assert!(
            doit_rattraper(&etat(ecart, false)),
            "{ecart:?} aurait dû déclencher une installation"
        );
    }
}

/// À jour : jouer ne télécharge rien. C'est le cas courant, et celui qui doit
/// rester instantané — sans quoi le bouton unique serait une régression pour
/// tout le monde sauf le jour d'une mise à jour.
#[test]
fn un_pack_a_jour_ne_declenche_rien() {
    assert!(!doit_rattraper(&etat(Ecart::AJour, false)));
}

/// Hors ligne, on ne rattrape RIEN, quel que soit l'écart affiché.
///
/// `Ecart::Inconnu` ne veut pas dire « à jour », il veut dire « on ne sait
/// pas ». Installer sur cette base repartirait du cache pour reposer ce qui
/// est déjà là — plusieurs minutes de vérification d'empreintes, sans rien
/// apprendre, au moment précis où le joueur n'a pas de réseau et veut
/// seulement jouer.
#[test]
fn hors_ligne_on_ne_rattrape_rien() {
    for ecart in [
        Ecart::Inconnu,
        Ecart::Absent,
        Ecart::MiseAJour,
        Ecart::Reinstallation,
        Ecart::AJour,
    ] {
        assert!(
            !doit_rattraper(&etat(ecart, true)),
            "{ecart:?} a déclenché une installation hors ligne"
        );
    }
}

/// Sans installation, on ne prétend pas savoir ce qu'une résolution qui n'a pas
/// eu lieu aurait trouvé.
///
/// Rendre une liste vide plutôt qu'une liste d'avant est ce qui empêche le
/// bandeau « mods introuvables » de rester affiché après un lancement où l'on
/// n'a rien résolu.
#[test]
fn sans_installation_rien_n_est_introuvable() {
    let deroulement = Deroulement {
        etat: etat(Ecart::AJour, false),
        installation: None,
        partie: None,
    };

    assert!(deroulement.introuvables().is_empty());
    assert!(deroulement.ecarts().is_empty());
}

/// Ce qu'une installation a trouvé REMONTE jusqu'au compte rendu.
///
/// Le test voisin ne couvrait que l'absence d'installation — donc les deux
/// listes vides — et les deux accesseurs pouvaient rendre `vec![]` en toute
/// circonstance sans qu'un test bronche. Or ce sont eux qui alimentent le
/// bandeau « mods introuvables » et le repli des écarts : les faire taire
/// laisserait un joueur jouer avec un pack incomplet sans que rien ne le lui
/// dise, ce qui est exactement la situation qu'un launcher de serveur moddé
/// doit empêcher.
#[test]
fn ce_qu_une_installation_a_trouve_remonte() {
    let mut lock = crate::essais::verrou(Vec::new());
    lock.unresolved = vec![
        crate::essais::manque("jei", "le manifeste"),
        crate::essais::manque("jade", "jei"),
    ];

    let deroulement = Deroulement {
        etat: etat(Ecart::AJour, false),
        installation: Some(crate::Outcome {
            instance: mc_instance::Instance {
                name: "samflix".into(),
                dir: "/nulle-part".into(),
                game_dir: "/nulle-part/minecraft".into(),
            },
            server_dir: "/nulle-part/server".into(),
            java: mc_java::Java {
                path: "/nulle-part/java".into(),
                version: mc_java::Version {
                    major: 21,
                    full: "21.0.5".into(),
                },
                origin: mc_java::Origin::Managed,
            },
            neoforge: "21.1.250".into(),
            assets_downloaded: 0,
            libraries: 0,
            client_mods: 0,
            server_mods: 0,
            removed: Vec::new(),
            lock,
            lock_path: "/nulle-part/samflix.lock.json".into(),
            previous_lock: None,
            source: "essai".into(),
            from_cache: false,
            ecarts: vec!["jei : 4.28.0 au lieu de 4.27.0".into()],
            purge: crate::etat::Purge::default(),
        }),
        partie: None,
    };

    assert_eq!(deroulement.introuvables(), vec!["jei", "jade"]);
    assert_eq!(
        deroulement.ecarts(),
        vec!["jei : 4.28.0 au lieu de 4.27.0".to_string()]
    );
}
