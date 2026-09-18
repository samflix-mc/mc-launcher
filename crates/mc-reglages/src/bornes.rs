//! Ce qu'un réglage a le droit de valoir.
//!
//! ## Pourquoi valider en Rust et non dans le formulaire
//!
//! Le formulaire valide déjà — un `<input type="range">` ne rend pas une
//! valeur hors bornes. Mais le fichier, lui, s'édite à la main, et c'est
//! précisément ce qu'un joueur qui cherche à gagner des images par seconde
//! fera. Une distance de rendu à 200 tronçons ne fait pas planter le jeu : elle
//! le fait allouer des gigaoctets jusqu'à l'`OutOfMemoryError`, vingt minutes
//! plus tard, sans que rien ne relie la cause à l'effet.
//!
//! On ramène donc dans les bornes plutôt que de refuser : un fichier hors
//! bornes ne doit pas empêcher de jouer.

use crate::types::{Apparence, Fenetre, Jeu, Lanceur, Reglages};

/// Distance de rendu : 2 tronçons au minimum — en dessous, le joueur ne voit
/// pas le sol sous ses pieds au chargement.
pub const RENDU: (u8, u8) = (2, 32);

/// Distance de simulation : 5 au minimum, c'est le plancher du jeu lui-même.
/// En dessous, les entités cessent de se mettre à jour autour du joueur.
pub const SIMULATION: (u8, u8) = (5, 32);

/// Images par seconde : au-delà de 260, Minecraft écrit `max` et ne limite
/// plus du tout. En deçà de 10, le jeu devient injouable sans qu'on sache
/// pourquoi.
pub const FPS: (u16, u16) = (10, 260);

/// Échelle de l'interface : 0 vaut « automatique », sinon 1 à 4. Une valeur
/// plus haute que 4 rend l'inventaire plus grand que l'écran.
pub const ECHELLE: (u8, u8) = (0, 4);

/// Mémoire de la JVM, en mégaoctets. 1 Go est le plancher sous lequel un
/// modpack ne démarre pas ; 32 Go le plafond au-delà duquel on alloue plus que
/// la machine n'a, et le système se met à échanger sur le disque.
pub const MEMOIRE: (u32, u32) = (1024, 32768);

/// Taille de la fenêtre du jeu. Le plancher est celui sous lequel l'interface
/// de Minecraft se chevauche.
pub const LARGEUR: (u32, u32) = (640, 7680);
pub const HAUTEUR: (u32, u32) = (480, 4320);

/// Le plancher du voile.
///
/// **Une mesure, pas un choix.** C'est la valeur en dessous de laquelle le
/// texte de l'interface cesse de tenir le contraste minimal sur l'image
/// embarquée la plus claire.
///
/// Elle est à REMESURER si l'on change le jeu d'images de fond : la valeur
/// actuelle vaut pour celles d'aujourd'hui, et une image plus claire la
/// rendrait insuffisante sans qu'aucun test ne le voie.
pub const VOILE_PLANCHER: f32 = 0.35;

fn borner<T: PartialOrd>(valeur: T, bornes: (T, T)) -> T {
    if valeur < bornes.0 {
        bornes.0
    } else if valeur > bornes.1 {
        bornes.1
    } else {
        valeur
    }
}

impl Reglages {
    /// Ramène tout dans les bornes. Ne refuse jamais.
    pub fn valider(&mut self) {
        self.schema = crate::types::SCHEMA;
        self.jeu.valider();
        self.fenetre.valider();
        self.lanceur.valider();
        self.apparence.valider();
    }
}

impl Jeu {
    pub fn valider(&mut self) {
        self.render_distance = borner(self.render_distance, RENDU);
        self.simulation_distance = borner(self.simulation_distance, SIMULATION);
        self.max_fps = borner(self.max_fps, FPS);
        self.gui_scale = borner(self.gui_scale, ECHELLE);
    }
}

impl Fenetre {
    pub fn valider(&mut self) {
        self.largeur = borner(self.largeur, LARGEUR);
        self.hauteur = borner(self.hauteur, HAUTEUR);
    }
}

impl Lanceur {
    pub fn valider(&mut self) {
        // `None` est laissé tel quel : c'est un choix — « laisse la JVM
        // décider » — et non une valeur hors bornes.
        self.memoire_mo = self.memoire_mo.map(|mo| borner(mo, MEMOIRE));
    }
}

impl Apparence {
    pub fn valider(&mut self) {
        // Un NaN passerait à travers une comparaison ordinaire : `NaN < x` et
        // `NaN > x` sont faux tous les deux, et la valeur ressortirait
        // inchangée. Elle rendrait alors un voile transparent, donc une
        // interface illisible — exactement ce que le plancher existe pour
        // empêcher.
        if !self.voile.is_finite() {
            self.voile = Apparence::default().voile;
            return;
        }
        self.voile = borner(self.voile, (VOILE_PLANCHER, 1.0));
    }
}

#[cfg(test)]
#[path = "bornes.test.rs"]
mod tests;
