//! Deux branches réclament le même projet : laquelle impose son build ?

use crate::jar::Side;
use crate::resolve::plan::Installed;
use crate::resolve::raison::Reason;
use crate::Candidate;

/// Ce qu'il advient du projet déjà retenu quand une autre branche le réclame.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Arbitrage {
    /// Le build en place tient. Son côté couvre désormais les deux usages, et
    /// `reprendre` dit si le nouveau demandeur devient celui qui répond de sa
    /// présence — c'est lui que le verrou nommera.
    Conserver { cote: Side, reprendre: bool },
    /// Le nouveau build l'emporte : l'entrée est écrasée, et les dépendances
    /// de l'écarté n'ont plus de demandeur.
    Remplacer { cote: Side },
}

/// Tranche entre le build retenu et celui qui se présente.
///
/// Deux règles, et rien d'autre. Une demande moins autoritaire ne déloge
/// personne — c'est ce qui fait converger la résolution. Et deux demandes qui
/// désignent le même build ne se disputent pas : le téléchargement est le
/// même, seul le côté change.
///
/// Le côté rendu est toujours l'union des deux : un mod réclamé côté serveur
/// par une branche et côté client par une autre doit être présent des deux
/// côtés, quelle que soit celle qui l'emporte sur la version.
pub(super) fn arbitrer(
    entrante: u8,
    retenue: u8,
    meme_build: bool,
    cote_retenu: Side,
    cote_entrant: Side,
) -> Arbitrage {
    let cote = cote_retenu.union(cote_entrant);
    if entrante <= retenue || meme_build {
        Arbitrage::Conserver {
            cote,
            reprendre: entrante > retenue,
        }
    } else {
        Arbitrage::Remplacer { cote }
    }
}

/// Applique l'arbitrage à l'entrée déjà en place.
///
/// Rend `None` quand le build en place tient — il n'y a alors plus rien à
/// faire pour cette demande —, et le côté à inscrire quand il faut l'écraser.
pub(super) fn confronter(
    existing: &mut Installed,
    candidate: &Candidate,
    reason: &Reason,
    entrante: u8,
    cote: Side,
    meme_build: bool,
) -> Option<Side> {
    match arbitrer(entrante, existing.autorite, meme_build, existing.side, cote) {
        Arbitrage::Conserver { cote, reprendre } => {
            existing.side = cote;
            if reprendre {
                existing.autorite = entrante;
                existing.reason = reason.clone();
            }
            None
        }
        Arbitrage::Remplacer { cote } => {
            annoncer_remplacement(candidate, existing, reason);
            Some(cote)
        }
    }
}

/// Dit au joueur quelle version il aura, et laquelle il n'aura pas.
///
/// Se taire reviendrait à installer le build de l'autre branche en silence :
/// le pack n'aurait pas la version que le manifeste promet, et rien ne le
/// signalerait avant le premier symptôme en jeu.
pub(super) fn annoncer_remplacement(retenu: &Candidate, ecarte: &Installed, reason: &Reason) {
    tracing::warn!(
        slug = %retenu.slug,
        ecartee = %ecarte.candidate.version_number,
        retenue = %retenu.version_number,
        "« {} » : {} impose {}, qui remplace {} retenue jusqu'ici",
        retenu.slug,
        reason.describe(),
        retenu.version_number,
        ecarte.candidate.version_number,
    );
}

#[cfg(test)]
#[path = "arbitrage.test.rs"]
mod tests;
