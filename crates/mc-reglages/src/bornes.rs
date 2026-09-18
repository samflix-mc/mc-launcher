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
/// modpack ne démarre pas.
///
/// Le plafond était à 32 Go, sur l'idée qu'au-delà on alloue plus que la
/// machine n'a. L'idée était juste, le nombre non : une machine de 64 Go — ce
/// que Sam a — voyait son curseur s'arrêter à 32, sans que rien ne dise
/// pourquoi. Un plafond fixe ne peut pas tenir ce raisonnement ; seul le
/// total de la machine le pourrait, et le launcher ne le connaît pas encore.
///
/// 64 Go est donc une borne de SÉCURITÉ et non un conseil : elle empêche une
/// valeur absurde d'arriver jusqu'à la JVM, sans prétendre savoir ce que la
/// machine porte. Le jour où le total sera lu — voir la tâche R6 — c'est lui
/// qui bornera, et cette constante redeviendra un simple garde-fou.
pub const MEMOIRE: (u32, u32) = (1024, 65_536);

/// Taille de la fenêtre du jeu. Le plancher est celui sous lequel l'interface
/// de Minecraft se chevauche.
pub const LARGEUR: (u32, u32) = (640, 7680);
pub const HAUTEUR: (u32, u32) = (480, 4320);

/// Le plancher du voile.
///
/// **Il vaut zéro, et c'est une décision de conception — pas un renoncement.**
///
/// Les deux versions précédentes valaient 0,35 puis 0,44. Toutes les deux
/// répondaient à la même question : « à partir de quelle opacité le texte
/// tient-il 4,5:1 sur l'image la plus claire concevable ? » La réponse était
/// juste ; c'était la QUESTION qui ne l'était plus.
///
/// Elle supposait que le contraste se règle en assombrissant l'image. Le design
/// system y répond autrement, et mieux : « sur une image claire, montez la base
/// du verre à 60 % du fond sur la racine de la fenêtre plutôt que d'assombrir
/// le texte ». Autrement dit, ce qu'on épaissit est le PANNEAU, pas l'image.
///
/// La différence n'est pas théorique. Un voile à 0,44 s'applique partout, y
/// compris là où il n'y a aucun texte — c'est-à-dire sur le milieu de l'écran,
/// que le design system laisse vide exprès pour que l'image se voie. Le
/// launcher affichait donc un rectangle noir à l'endroit même que toute sa
/// direction artistique existe pour montrer.
///
/// La garantie de contraste n'a pas disparu, elle a changé de couche. Elle est
/// tenue en trois endroits, tous dans `web/src/` :
///
/// 1. `.hm-stage__art::after`, le dégradé du design system — le fond à 52 % en
///    haut, 36 % au milieu, 80 % en bas. C'est lui qui fait que la barre du bas
///    se lit quelle que soit l'image, et il n'est pas réglable.
/// 2. `--glass-base`, qui s'ÉPAISSIT quand le voile s'amincit : de 46 % du fond
///    à 72 %. À voile nul et sur une image blanche, `ink-3` — la teinte la plus
///    claire que le design system autorise — tient encore 4,5:1 sur un panneau.
///    C'est la parade que le design system prescrit, appliquée automatiquement.
/// 3. Les ombres portées des textes qui n'ont pas de panneau sous eux : les
///    titres de tuile et l'indication du bouton de jeu.
///
/// Le voile redevient donc ce qu'un réglage d'apparence doit être : une
/// préférence, qui ne peut pas rendre l'interface illisible parce que ce n'est
/// plus lui qui la rend lisible.
///
/// À rouvrir si l'on retire la compensation de `--glass-base` — et dans ce cas,
/// c'est la mesure du point 2 qu'il faut refaire, pas celle d'avant.
pub const VOILE_PLANCHER: f32 = 0.0;

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
