//! Le chemin complet, tel que la fenêtre le montre.
//!
//! `mc_pack::Etape` couvre l'installation, et rien d'autre : la connexion
//! Microsoft et la vérification de licence n'en font pas partie — la ligne de
//! commande n'authentifie qu'au lancement, et `mc-pack` ne vérifie aucune
//! licence. L'application, elle, enchaîne les trois, et doit les montrer d'une
//! seule pièce.
//!
//! D'où cette énumération plus large, qui encadre celle de `mc-pack` sans la
//! remplacer : deux phases avant, deux après.
//!
//! Le chemin est affiché **en entier dès le départ**, chaque phase portant son
//! état. C'est ce qui distingue « on en est à la moitié » de « il se passe
//! quelque chose » : un joueur qui voit les sept étapes restantes sait ce qu'il
//! attend, là où une seule ligne qui change ne dit rien de la durée.

use serde::Serialize;

/// Une phase de la cinématique, de l'ouverture de la fenêtre au jeu lancé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// Session Microsoft : reprise, ou ouverte par code d'appareil.
    Connexion,
    /// Le compte possède-t-il Minecraft Java Edition ?
    Licence,
    /// Lecture du manifeste du pack, et du verrou.
    Pack,
    /// Quelle version de NeoForge.
    Chargeur,
    /// Les fichiers de Mojang : client, bibliothèques, assets.
    Minecraft,
    /// Le runtime Java, détecté ou installé.
    Java,
    /// L'installateur NeoForge.
    NeoForge,
    /// Résolution, téléchargement et répartition des mods.
    Mods,
    /// Le verrou, écrit en dernier.
    Verrou,
    /// Tout est en place : le bouton « Jouer » s'allume.
    Pret,
    /// Le jeu tourne.
    Lancement,
}

impl Phase {
    /// Toutes les phases, dans l'ordre où elles surviennent.
    pub const TOUTES: [Phase; 11] = [
        Phase::Connexion,
        Phase::Licence,
        Phase::Pack,
        Phase::Chargeur,
        Phase::Minecraft,
        Phase::Java,
        Phase::NeoForge,
        Phase::Mods,
        Phase::Verrou,
        Phase::Pret,
        Phase::Lancement,
    ];

    /// Ce que la fenêtre écrit à côté de la phase.
    ///
    /// En français et destiné à être lu : contrairement à l'identifiant
    /// sérialisé, ce texte peut changer sans rien casser.
    pub fn libelle(self) -> &'static str {
        match self {
            Phase::Connexion => "Compte Microsoft",
            Phase::Licence => "Licence Minecraft",
            Phase::Pack => "Pack",
            Phase::Chargeur => "Version de NeoForge",
            Phase::Minecraft => "Fichiers du jeu",
            Phase::Java => "Java",
            Phase::NeoForge => "Chargeur NeoForge",
            Phase::Mods => "Mods",
            Phase::Verrou => "Verrou",
            Phase::Pret => "Prêt à jouer",
            Phase::Lancement => "Jeu lancé",
        }
    }

    /// Le rang de la phase, à partir de zéro.
    pub fn rang(self) -> usize {
        Phase::TOUTES
            .iter()
            .position(|phase| *phase == self)
            .expect("toute phase est dans TOUTES")
    }
}

/// Les étapes de `mc-pack` prennent place au milieu du chemin.
impl From<mc_pack::Etape> for Phase {
    fn from(etape: mc_pack::Etape) -> Self {
        match etape {
            mc_pack::Etape::Pack => Phase::Pack,
            mc_pack::Etape::Chargeur => Phase::Chargeur,
            mc_pack::Etape::Minecraft => Phase::Minecraft,
            mc_pack::Etape::Java => Phase::Java,
            mc_pack::Etape::NeoForge => Phase::NeoForge,
            mc_pack::Etape::Mods => Phase::Mods,
            mc_pack::Etape::Verrou => Phase::Verrou,
        }
    }
}

#[cfg(test)]
#[path = "phase.test.rs"]
mod tests;
