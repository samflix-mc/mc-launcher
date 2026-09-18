//! Ce qu'une installation raconte pendant qu'elle travaille.
//!
//! Le compte rendu final ne sert qu'à celui qui a attendu jusqu'au bout. Entre
//! le premier octet et lui, il se passe plusieurs minutes, et quelqu'un
//! regarde.
//!
//! ## Pourquoi une étape, et pas seulement une ligne de texte
//!
//! La première version de ce passage était `&dyn Fn(&str)` : l'installation
//! poussait des phrases déjà mises en forme, et la ligne de commande les
//! imprimait. Cela suffit à un terminal, qui empile les lignes. Cela ne suffit
//! pas à une fenêtre, qui doit savoir *où l'on en est* — quelle étape est
//! finie, laquelle travaille, lesquelles restent — pour dessiner autre chose
//! qu'un journal qui défile. Reconstituer cela en relisant des phrases
//! reviendrait à analyser sa propre sortie, et la moindre reformulation
//! casserait l'affichage sans casser la compilation.
//!
//! Les deux coexistent donc, et ne disent pas la même chose : [`Rapport::etape`]
//! porte la structure, [`Rapport::note`] le détail que seul un humain lit.
//!
//! ## Et le téléchargement
//!
//! [`Rapport::telechargement`] reçoit ce que `mc-dl` émet — octets, fichier en
//! cours, lots annoncés. `mc-pack` se contente de brancher le fil : il pose
//! l'observateur sur son client HTTP et sur celui des mods, et ne regarde pas
//! ce qui y passe.

/// Où en est une installation.
///
/// L'ordre de la déclaration est celui des étapes, et [`Etape::TOUTES`] en
/// dépend : c'est ce qui permet à un affichage de montrer le chemin entier dès
/// le départ, plutôt que de découvrir les étapes une par une.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Etape {
    /// Lecture du manifeste, et du verrou s'il y en a un.
    Pack,
    /// Quelle version de NeoForge — résolue avant tout le reste, pour que le
    /// verrou consigne un numéro et non le mot « latest ».
    Chargeur,
    /// Les fichiers de Mojang : le client, ses bibliothèques, ses assets.
    Minecraft,
    /// Le runtime Java, détecté ou installé.
    Java,
    /// L'installateur NeoForge, qui patche le client vanilla.
    NeoForge,
    /// Résolution, téléchargement et répartition des mods.
    Mods,
    /// Le verrou, écrit en dernier : il décrit ce qui a réellement été fait.
    Verrou,
}

impl Etape {
    /// Toutes les étapes, dans l'ordre où elles surviennent.
    pub const TOUTES: [Etape; 7] = [
        Etape::Pack,
        Etape::Chargeur,
        Etape::Minecraft,
        Etape::Java,
        Etape::NeoForge,
        Etape::Mods,
        Etape::Verrou,
    ];

    /// Un identifiant stable, destiné à être lu par une machine.
    ///
    /// Pas un libellé : ce qui s'affiche se traduit et se reformule, ce qui
    /// s'écrit ici sert de clé et ne doit pas bouger.
    pub fn as_str(self) -> &'static str {
        match self {
            Etape::Pack => "pack",
            Etape::Chargeur => "chargeur",
            Etape::Minecraft => "minecraft",
            Etape::Java => "java",
            Etape::NeoForge => "neoforge",
            Etape::Mods => "mods",
            Etape::Verrou => "verrou",
        }
    }

    /// Le rang de l'étape dans la séquence, à partir de zéro.
    pub fn rang(self) -> usize {
        Etape::TOUTES
            .iter()
            .position(|etape| *etape == self)
            .expect("toute étape est dans TOUTES")
    }
}

/// À qui l'installation rend compte.
///
/// `Send + Sync + 'static` parce que l'observateur de téléchargement que
/// `mc-pack` en dérive traverse les tâches concurrentes de `mc-dl`.
pub trait Rapport: Send + Sync + 'static {
    /// Une étape commence.
    fn etape(&self, etape: Etape);

    /// Une ligne de détail, destinée à être lue telle quelle.
    fn note(&self, texte: &str);

    /// Un téléchargement avance.
    ///
    /// Ignoré par défaut : un terminal n'en fait rien, et le débit y défile
    /// plus vite qu'il ne se lit.
    /// Hors de portée des tests de mutation : cette méthode n'écrit que sur
    /// la sortie standard, que Rust ne sait pas relire depuis le processus qui
    /// l'émet. Ce qu'elle CALCULE — le débit, le temps restant, le pourcentage
    /// — est produit par `Suivi`, qui est éprouvé pour lui-même.
    #[mutants::skip]
    fn telechargement(&self, avancement: mc_dl::Avancement<'_>) {
        let _ = avancement;
    }

    /// La résolution des mods avance.
    ///
    /// **Distinct de [`Rapport::telechargement`], et c'est tout l'objet.** La
    /// résolution passe l'essentiel de son temps à interroger des API — une
    /// trentaine de secondes sur un pack de cinquante mods — pour des réponses
    /// de quelques kilooctets. Aucune barre d'octets ne bouge pendant ce
    /// temps, et l'écran ne se distingue pas d'un écran planté.
    ///
    /// `total` est une estimation qui peut grandir : une demande résolue peut
    /// en faire naître d'autres.
    ///
    /// Ignoré par défaut, comme le téléchargement : un terminal n'en fait rien.
    #[mutants::skip]
    fn resolution(&self, faits: usize, total: usize) {
        let _ = (faits, total);
    }
}

/// Un rapport qui n'écoute rien.
///
/// Pour les appelants qui veulent seulement le résultat — les tests, et tout
/// usage où personne ne regarde.
pub struct Muet;

impl Rapport for Muet {
    fn etape(&self, _etape: Etape) {}
    fn note(&self, _texte: &str) {}
}

#[cfg(test)]
#[path = "progression.test.rs"]
mod tests;
