//! La file d'attente de la résolution : ce qui reste à résoudre, et dans quel
//! ordre.

use crate::Origin;

use super::demande::Request;
use super::raison::Reason;

/// Clé d'unicité d'un projet : un mod ne peut être présent qu'une fois.
pub(super) type Cle = (Origin, String);

/// Une demande en attente de résolution.
pub(super) struct Demande {
    request: Request,
    reason: Reason,
    /// Le build qui a poussé cette demande, quand elle découle d'un autre.
    ///
    /// La clé, et non le titre que porte [`Reason::Declared`] : « Jade » se
    /// publie sous le même nom chez Modrinth et chez CurseForge, et les deux
    /// projets coexistent dans `chosen` jusqu'à la déduplication finale.
    /// Retirer les dépendances d'un build écarté par son titre emportait donc
    /// celles de son homonyme, que plus rien ne repoussait.
    parent: Option<Cle>,
}

impl Demande {
    /// Ce qu'il faut pour la résoudre. Le parent ne sert qu'à la file, qui le
    /// garde pour retirer les dépendances d'un build écarté.
    pub(super) fn into_parts(self) -> (Request, Reason) {
        (self.request, self.reason)
    }
}

/// File de résolution : le manifeste d'abord, les dépendances ensuite.
///
/// Une seule file suffisait tant qu'on ne regardait que le résultat. Mais elle
/// se vidait en pile : la dépendance d'une demande déjà dépilée passait avant
/// les demandes restantes. Un mod à la fois épinglé par le verrou et dépendance
/// d'un autre était donc résolu sans son épinglage, et la demande épinglée
/// arrivait sur une clé déjà prise.
#[derive(Default)]
pub(super) struct FileDeResolution {
    manifeste: Vec<Demande>,
    derivees: Vec<Demande>,
}

impl FileDeResolution {
    pub(super) fn pousser(&mut self, request: Request, reason: Reason, parent: Option<Cle>) {
        let demande = Demande {
            request,
            reason,
            parent,
        };
        match demande.reason {
            Reason::Explicit => self.manifeste.push(demande),
            _ => self.derivees.push(demande),
        }
    }

    /// Dépile en profondeur, mais jamais une dépendance tant que le manifeste
    /// n'est pas entièrement traité.
    pub(super) fn suivante(&mut self) -> Option<Demande> {
        self.manifeste.pop().or_else(|| self.derivees.pop())
    }

    /// Retire de la file les dépendances qu'un build avait déclarées.
    ///
    /// Appelé quand un build en remplace un autre : les dépendances du build
    /// écarté n'ont plus de demandeur. Les laisser ferait installer des jars
    /// que plus rien ne réclame — et le verrou les consignerait comme
    /// dépendances d'une version qui n'est pas celle retenue.
    ///
    /// Ne rattrape que ce qui est encore en file. Une dépendance de l'ancien
    /// build déjà résolue à un tour précédent y reste : la retirer demanderait
    /// de savoir qui d'autre s'appuie dessus, donc de tenir le graphe inverse.
    /// L'écart est borné — un jar de bibliothèque en trop, que NeoForge charge
    /// sans se plaindre — et sans commune mesure avec le défaut d'en face,
    /// qui était d'installer le mauvais build épinglé.
    pub(super) fn oublier_dependances_de(&mut self, parent: &Cle) {
        self.derivees
            .retain(|demande| demande.parent.as_ref() != Some(parent));
    }

    /// Combien de demandes attendent encore leur tour.
    ///
    /// Sert à annoncer une progression, et rien d'autre : le total qu'on en
    /// tire BOUGE, puisque résoudre une demande peut en faire naître d'autres.
    /// C'est assumé — une barre qui recule un peu vaut mieux qu'une fenêtre
    /// figée pendant les trente-six secondes que dure l'interrogation des API.
    pub(super) fn restantes(&self) -> usize {
        self.manifeste.len() + self.derivees.len()
    }

    pub(super) fn est_vide(&self) -> bool {
        self.manifeste.is_empty() && self.derivees.is_empty()
    }
}

#[cfg(test)]
#[path = "file.test.rs"]
mod tests;

#[cfg(test)]
#[path = "file.oubli.test.rs"]
mod tests_oubli;
