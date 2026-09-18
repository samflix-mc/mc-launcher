//! Ce que le joueur règle, et les bornes de chaque réglage.

use serde::{Deserialize, Serialize};

/// Version du format. Un fichier d'une autre version se relit avec les défauts
/// plutôt que de faire échouer le lancement : perdre ses réglages est ennuyeux,
/// ne pas pouvoir jouer l'est davantage.
pub const SCHEMA: u32 = 1;

/// Tout ce que le joueur peut changer.
///
/// Quatre sections, et leur découpage n'est pas cosmétique : il dit QUI lit
/// chaque valeur.
///
/// - [`Jeu`] — des clés fusionnées dans `options.txt`. C'est Minecraft qui les
///   lit, pas nous.
/// - [`Fenetre`] — la fenêtre du JEU, passée en arguments de lancement.
/// - [`Lanceur`] — ce que le launcher fait de lui-même au lancement.
/// - [`Apparence`] — la fenêtre du LAUNCHER. Le jeu n'en sait rien.
///
/// Confondre `fenetre` et `apparence` est l'erreur qu'on fait à chaque fois :
/// la première est celle du jeu, la seconde la nôtre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reglages {
    pub schema: u32,
    pub jeu: Jeu,
    pub fenetre: Fenetre,
    pub lanceur: Lanceur,
    pub apparence: Apparence,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            jeu: Jeu::default(),
            fenetre: Fenetre::default(),
            lanceur: Lanceur::default(),
            apparence: Apparence::default(),
        }
    }
}

/// Les clés que le launcher fusionne dans `options.txt`.
///
/// Uniquement celles qu'un joueur règle vraiment et qu'un modpack ne pilote
/// pas. `graphicsMode` en est exclu : les shaders le remplacent, et l'imposer
/// depuis le launcher défait ce qu'Iris a réglé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Jeu {
    /// Distance de rendu, en tronçons. 2 à 32.
    pub render_distance: u8,
    /// Distance de simulation, en tronçons. 5 à 32 — le plancher est celui du
    /// jeu, en dessous duquel les entités cessent de se mettre à jour autour
    /// du joueur.
    pub simulation_distance: u8,
    /// Plafond d'images par seconde. 10 à 260 — au-delà de 260, Minecraft
    /// écrit `max` et ne limite plus.
    pub max_fps: u16,
    /// Échelle de l'interface. 0 vaut « automatique » ; sinon 1 ou plus.
    pub gui_scale: u8,
    /// Synchronisation verticale.
    pub vsync: bool,
}

impl Default for Jeu {
    fn default() -> Self {
        Self {
            render_distance: 12,
            simulation_distance: 10,
            max_fps: 120,
            gui_scale: 0,
            vsync: true,
        }
    }
}

/// Comment la fenêtre DU JEU s'ouvre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModeFenetre {
    /// À la taille demandée.
    Fenetree,
    /// À la zone utile de l'écran, panneaux du bureau déduits.
    Maximisee,
    /// Plein écran exclusif.
    PleinEcran,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Fenetre {
    pub mode: ModeFenetre,
    /// La taille demandée en mode fenêtré. Ignorée dans les deux autres.
    pub largeur: u32,
    pub hauteur: u32,
}

impl Default for Fenetre {
    fn default() -> Self {
        Self {
            mode: ModeFenetre::Fenetree,
            largeur: 1280,
            hauteur: 720,
        }
    }
}

impl Fenetre {
    /// La valeur de la clé `fullscreen` d'`options.txt`.
    ///
    /// DÉRIVÉE du mode, et non un champ à part. C'est ce qui empêche les deux
    /// de diverger — et ils divergeraient : F11 bascule cette clé en cours de
    /// partie, et le jeu la persiste. Un champ `plein_ecran` indépendant du
    /// mode aurait fait du plein écran un interrupteur à sens unique, où
    /// l'appuyer depuis le jeu serait défait au lancement suivant.
    pub fn plein_ecran(&self) -> bool {
        self.mode == ModeFenetre::PleinEcran
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Lanceur {
    /// Mémoire allouée à la JVM, en mégaoctets.
    ///
    /// `None` laisse la JVM décider — un quart de la mémoire de la machine, ce
    /// qui ne suffit pas à un modpack et donne un `OutOfMemoryError` au bout de
    /// vingt minutes. Le défaut du launcher n'est donc PAS `None`.
    pub memoire_mo: Option<u32>,
    /// Réduire la fenêtre du launcher quand le jeu démarre.
    pub reduire_au_lancement: bool,
}

impl Default for Lanceur {
    fn default() -> Self {
        Self {
            memoire_mo: Some(4096),
            reduire_au_lancement: true,
        }
    }
}

/// Le fond de la fenêtre DU LAUNCHER.
///
/// Un identifiant énuméré et NON un chemin. Un chemin ferait du front le
/// propriétaire de la liste, et obligerait Rust à persister une valeur qu'il
/// ne sait pas valider — un chemin vers un fichier effacé, vers un répertoire,
/// vers l'extérieur du launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fond {
    Spawn,
    Nether,
    Fin,
    Uni,
}

impl Fond {
    /// Le nom du fichier, sous `public/fonds/`.
    pub fn fichier(self) -> &'static str {
        match self {
            Fond::Spawn => "spawn.webp",
            Fond::Nether => "nether.webp",
            Fond::Fin => "fin.webp",
            // Pas d'image du tout : un dégradé, pour qui trouve les
            // photographies bruyantes ou qui a un écran lent.
            Fond::Uni => "",
        }
    }

    pub const TOUS: [Fond; 4] = [Fond::Spawn, Fond::Nether, Fond::Fin, Fond::Uni];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Apparence {
    pub fond: Fond,
    /// L'opacité du voile posé entre l'image et l'interface.
    ///
    /// **Borné par le bas, et c'est la seule borne qui ne soit pas un
    /// confort.** En dessous du plancher, le texte de l'interface ne tient plus
    /// le contraste minimal sur l'image embarquée la plus claire : les libellés
    /// deviennent illisibles sur une partie de l'écran seulement, ce qui est la
    /// pire des façons de casser une interface — cela ressemble à un défaut de
    /// rendu et non à un réglage.
    ///
    /// Le curseur va donc du plancher à 1, et non de 0 à 1.
    pub voile: f32,
}

impl Default for Apparence {
    fn default() -> Self {
        Self {
            fond: Fond::Spawn,
            voile: 0.55,
        }
    }
}
