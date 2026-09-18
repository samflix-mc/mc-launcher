//! Les états que l'interface doit savoir montrer.
//!
//! ## Pourquoi des scénarios et pas la vraie chaîne
//!
//! Le but de ce serveur est de travailler l'INTERFACE. Or la plupart des états
//! qu'elle doit savoir dessiner sont pénibles ou impossibles à provoquer pour
//! de bon : une installation figée à trente-sept pour cent, un pack dont trois
//! mods sont introuvables, un compte sans licence Minecraft, un hôte hors
//! ligne. Les atteindre en vrai demanderait de casser quelque chose, et de le
//! réparer entre deux essais.
//!
//! Un scénario les rend tous accessibles en une requête. C'est ce qui permet
//! de regarder l'écran « mods introuvables » sans avoir à publier un pack
//! fautif.
//!
//! **Ce qui n'est PAS simulé** : `marque`, `chemin`, `reglages`,
//! `enregistrer_reglages`. Ces quatre-là appellent la VRAIE implémentation —
//! elles ne touchent ni au réseau ni à huit cents mégaoctets, et les voir
//! mentir n'apprendrait rien. `enregistrer_reglages` écrit donc pour de bon
//! dans `reglages.json`, ce qui est exactement ce qu'on veut éprouver : c'est
//! là que les bornes s'appliquent.

use serde::Serialize;

use crate::commandes::{Compte, EtapeVue};
use crate::phase::Phase;
use crate::suivi::Avancement;

/// L'état que le serveur sert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    /// Personne n'est connecté : l'écran doit s'ouvrir sur la connexion.
    Deconnecte,
    /// Compte Microsoft valide, mais sans licence Minecraft. Un ÉTAT à
    /// afficher, pas une redirection.
    SansLicence,
    /// Connecté, rien d'installé : le bouton dit INSTALLER.
    RienInstalle,
    /// Connecté, pack posé et conforme : le bouton dit JOUER.
    PretAJouer,
    /// Pack posé mais le verrou publié a changé : JOUER, avec rattrapage.
    ARattraper,
    /// L'hôte du pack n'a pas répondu : l'écart est inconnu.
    HorsLigne,
    /// Une installation est en cours, figée à mi-parcours.
    EnInstallation,
    /// L'installation s'est faite, mais trois mods manquent.
    ModsIntrouvables,
}

impl Etat {
    /// Tous les scénarios, avec le nom par lequel on les demande.
    pub const TOUS: [(&'static str, Etat); 8] = [
        ("deconnecte", Etat::Deconnecte),
        ("sans-licence", Etat::SansLicence),
        ("rien-installe", Etat::RienInstalle),
        ("pret-a-jouer", Etat::PretAJouer),
        ("a-rattraper", Etat::ARattraper),
        ("hors-ligne", Etat::HorsLigne),
        ("en-installation", Etat::EnInstallation),
        ("mods-introuvables", Etat::ModsIntrouvables),
    ];

    pub fn depuis_nom(nom: &str) -> Option<Etat> {
        Etat::TOUS
            .iter()
            .find(|(connu, _)| *connu == nom)
            .map(|(_, etat)| *etat)
    }

    /// Le compte, ou `None` quand personne n'est connecté.
    pub(crate) fn compte(self) -> Option<Compte> {
        match self {
            Etat::Deconnecte => None,
            Etat::SansLicence => Some(Compte {
                pseudo: "SansLicence".to_string(),
                uuid: "00000000-0000-0000-0000-00000000dead".to_string(),
                possede_le_jeu: false,
            }),
            _ => Some(Compte {
                pseudo: "thesam1798".to_string(),
                uuid: "9f6e4a0c-1b2d-4e3f-8a9b-0c1d2e3f4a5b".to_string(),
                possede_le_jeu: true,
            }),
        }
    }

    /// Ce que le disque et le pack publié disent.
    pub(crate) fn pack(self) -> mc_pack::EtatDuPack {
        use mc_pack::comparaison::{Action, Ecart};

        let (action, ecart, installe, hors_ligne) = match self {
            Etat::Deconnecte | Etat::SansLicence | Etat::RienInstalle => {
                (Action::Installer, Ecart::Absent, false, false)
            }
            Etat::PretAJouer | Etat::ModsIntrouvables => (Action::Jouer, Ecart::AJour, true, false),
            Etat::ARattraper | Etat::EnInstallation => {
                (Action::Jouer, Ecart::MiseAJour, true, false)
            }
            Etat::HorsLigne => (Action::Jouer, Ecart::Inconnu, true, true),
        };

        mc_pack::EtatDuPack {
            action,
            ecart,
            hors_ligne,
            installe,
            nom: Some("samflix".to_string()),
            version: Some("0.1.0".to_string()),
            java: Some(21),
            mods: 128,
            generation: 0,
        }
    }

    /// L'avancement affiché à l'ouverture.
    ///
    /// C'est ce qui distingue « rien ne se passe » de « une installation est
    /// en cours » : l'interface doit savoir dessiner les deux, et l'on ne peut
    /// pas attendre qu'une vraie installation atteigne trente-sept pour cent
    /// pour regarder à quoi elle ressemble.
    pub(crate) fn avancement(self) -> Avancement {
        match self {
            Etat::EnInstallation => Avancement {
                phase: Phase::Mods,
                achevee: false,
                note: Some("128 mods, dont 43 ajoutés par dépendance".to_string()),
                fichier: Some("sodium-neoforge-0.6.13.jar".to_string()),
                octets: 312_000_000,
                total: 840_000_000,
                fichiers: 47,
                fichiers_total: 128,
                actif: true,
                debit: 8_400_000,
                restant: Some(63),
            },
            Etat::Deconnecte | Etat::SansLicence => Avancement {
                phase: Phase::Connexion,
                achevee: false,
                ..vide()
            },
            Etat::PretAJouer | Etat::ARattraper | Etat::HorsLigne | Etat::ModsIntrouvables => {
                Avancement {
                    phase: Phase::Pret,
                    achevee: true,
                    ..vide()
                }
            }
            Etat::RienInstalle => Avancement {
                phase: Phase::Licence,
                achevee: true,
                ..vide()
            },
        }
    }

    /// Ce qu'une partie rend, une fois le jeu refermé.
    pub(crate) fn partie(self, avec_partie: bool) -> crate::commandes::pack::CompteRendu {
        crate::commandes::pack::CompteRendu {
            verdict: if avec_partie {
                "Partie terminée.".to_string()
            } else if matches!(self, Etat::ARattraper) {
                "Le pack est installé.".to_string()
            } else {
                "Le pack était déjà à jour : rien à poser.".to_string()
            },
            rattrapee: matches!(self, Etat::ARattraper),
            introuvables: match self {
                Etat::ModsIntrouvables => vec![
                    "journeymap".to_string(),
                    "waystones".to_string(),
                    "litematica".to_string(),
                ],
                _ => Vec::new(),
            },
            ecarts: match self {
                Etat::ARattraper => vec!["jei : 19.21.1.317 au lieu de 19.21.0.247".to_string()],
                _ => Vec::new(),
            },
            hors_ligne: matches!(self, Etat::HorsLigne),
            purge: Vec::new(),
        }
    }

    /// Le fil de nouvelles.
    ///
    /// Non vide dans tous les scénarios sauf `hors-ligne` : c'est la page
    /// qu'on ne peut PAS regarder aujourd'hui, puisque les trois hôtes rendent
    /// 404. Sans ces billets, elle resterait invisible tant que mc-content
    /// n'a pas publié.
    pub(crate) fn fil(self) -> mc_nouvelles::Fil {
        let billets = if matches!(self, Etat::HorsLigne) {
            Vec::new()
        } else {
            billets_de_demonstration()
        };

        mc_nouvelles::Fil {
            billets,
            hors_ligne: matches!(self, Etat::HorsLigne),
            ecartes: Vec::new(),
        }
    }
}

fn vide() -> Avancement {
    Avancement {
        phase: Phase::Connexion,
        achevee: false,
        note: None,
        fichier: None,
        octets: 0,
        total: 0,
        fichiers: 0,
        fichiers_total: 0,
        actif: false,
        debit: 0,
        restant: None,
    }
}

/// Deux billets, dont un illustré, et un qui exerce tout le markdown reconnu.
///
/// Le second existe pour que la page soit regardée avec du CONTENU réel :
/// titres, listes, gras, code, lien, séparateur. Une page de nouvelles à un
/// paragraphe ne montre pas ses défauts de mise en forme.
fn billets_de_demonstration() -> Vec<mc_nouvelles::Billet> {
    let analyser = |corps: &str| mc_nouvelles::analyse::analyser(corps, "https://exemple.invalid/");

    vec![
        mc_nouvelles::Billet {
            id: "2026-09-ouverture".to_string(),
            titre: "Le launcher est là".to_string(),
            date: "2026-09-18T18:00:00Z".to_string(),
            epinglee: true,
            image: None,
            corps: analyser(
                "Le launcher installe le pack et lance le jeu **en un seul geste**.\n\n\
                 Ce qu'il faut savoir :\n\n\
                 - il installe ce que le serveur charge, à la version près\n\
                 - il pose le Java qu'il faut, sans toucher à celui du système\n\
                 - il ne réinstalle rien tant que le pack n'a pas bougé\n\n\
                 Les réglages sont dans `Configuration`.",
            ),
        },
        mc_nouvelles::Billet {
            id: "2026-09-regles".to_string(),
            titre: "Les règles du serveur".to_string(),
            date: "2026-09-17T12:00:00Z".to_string(),
            epinglee: false,
            image: None,
            corps: analyser(
                "Trois règles, et elles tiennent en une ligne chacune.\n\n\
                 ## Le respect\n\n\
                 Pas d'insulte, pas de harcèlement. C'est la seule qui mène à un \
                 bannissement immédiat.\n\n\
                 ## Les constructions\n\n\
                 On ne casse pas chez les autres. Un `/back` mal placé n'est pas une \
                 excuse.\n\n\
                 ---\n\n\
                 Le détail est sur [la page des règles](https://exemple.invalid/regles).",
            ),
        },
    ]
}

/// Ce que le serveur annonce de lui-même, à la racine.
#[derive(Serialize)]
pub(crate) struct Accueil {
    pub(crate) scenario: String,
    pub(crate) scenarios: Vec<String>,
    pub(crate) commandes: Vec<String>,
}

/// Le chemin complet des phases, tel que la vraie commande le rend.
pub(crate) fn chemin() -> Vec<EtapeVue> {
    crate::commandes::chemin()
}
