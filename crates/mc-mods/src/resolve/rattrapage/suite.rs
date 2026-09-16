//! Que faire de ce que la recherche a rendu.

use std::collections::BTreeSet;

use crate::jar::Side;
use crate::resolve::demande::Request;
use crate::resolve::file::Cle;
use crate::resolve::plan::Unresolved;
use crate::resolve::raison::Reason;
use crate::{Candidate, Channel};

/// pourrait la fournir.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Rattrapage {
    /// Un build fournit le `modId` : sa demande rejoint la file.
    Demander(Request, Reason),
    /// Personne ne le fournira par cette voie ; le manque est consigné et
    /// l'installation continue — le jeu démarrera sans, ou pas du tout, mais
    /// c'est à l'appelant d'en juger, pas au résolveur.
    Renoncer(Unresolved),
}

/// Décide sans rien demander au réseau, et c'est tout l'intérêt : la recherche
/// du fournisseur est faite, ce qui reste est un jugement sur ce qu'elle a
/// rendu — jugement dont dépend la terminaison de la résolution.
pub(super) fn suite_du_rattrapage(
    trouve: Option<Candidate>,
    impasses: &BTreeSet<Cle>,
    mod_id: String,
    required_by: String,
    side: Side,
) -> Rattrapage {
    // Le projet a déjà perdu cet arbitrage : le reproposer relancerait un tour
    // identique, jusqu'à épuisement de [`MAX_PASSES`].
    let impasse = trouve
        .as_ref()
        .is_some_and(|c| impasses.contains(&(c.origin, c.project_id.clone())));

    match trouve {
        Some(candidate) if !impasse => Rattrapage::Demander(
            Request {
                slug: candidate.project_id.clone(),
                source: Some(candidate.origin),
                file: Some(candidate.version_id.clone()),
                version: None,
                side: Some(side),
                channel: Some(Channel::Beta),
                expected_sha1: None,
                expected_sha512: None,
            },
            Reason::Implicit {
                by: required_by,
                mod_id,
            },
        ),
        autre => {
            match &autre {
                Some(candidate) => tracing::error!(
                    mod_id = %mod_id,
                    exige_par = %required_by,
                    projet = %candidate.slug,
                    "« {mod_id} », exigé par {required_by}, ne serait fourni que par \
                     « {} » — dont la place est tenue par un build qu'une demande plus \
                     autoritaire impose",
                    candidate.slug
                ),
                None => tracing::error!(
                    mod_id = %mod_id,
                    exige_par = %required_by,
                    "« {mod_id} », exigé par {required_by}, est introuvable sur toutes les sources"
                ),
            }
            Rattrapage::Renoncer(Unresolved {
                mod_id,
                required_by,
                side,
            })
        }
    }
}

/// Le build à retenir pour une demande, ou l'explication de son absence.
///
/// Sortie de [`resolve_with`], dont elle représentait le gros du volume : trois
/// impasses — version demandée introuvable, projet introuvable, téléchargement
/// interdit par l'auteur — qui n'ont besoin que de la demande pour être

#[cfg(test)]
#[path = "suite.test.rs"]
mod tests;
