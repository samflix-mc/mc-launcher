//! Chercher qui fournit un `modId` qu'aucune API n'annonçait.

mod suite;

use anyhow::Result;
use std::collections::BTreeSet;

use crate::Channel;
use crate::jar::Side;

use super::demande::Request;
use super::registre::filtre::pick;

use super::Registry;
use super::file::{Cle, FileDeResolution};
use super::plan::Unresolved;

use suite::{Rattrapage, suite_du_rattrapage};

/// Cherche un fournisseur pour chaque `modId` qu'aucune API n'annonçait.
///
/// Sortie de [`resolve_with`] : ce rattrapage ne touche ni à la table des
/// retenus ni au décompte des tours, il ne fait qu'alimenter la file — ou la
/// liste des manques, quand personne ne peut fournir.
pub(super) async fn rattraper(
    registry: &Registry,
    manques: Vec<(String, String, Side)>,
    mc: &str,
    loader: &str,
    impasses: &BTreeSet<Cle>,
    queue: &mut FileDeResolution,
    unresolved: &mut Vec<Unresolved>,
) -> Result<()> {
    for (mod_id, required_by, side) in manques {
        // La trace la plus utile du lot : elle nomme une dépendance que ni le
        // manifeste ni l'API n'annonçaient, et sans laquelle le jeu ne
        // démarrerait pas.
        tracing::info!(
            mod_id = %mod_id,
            exige_par = %required_by,
            "Dépendance implicite : {required_by} exige « {mod_id} », qu'aucune API ne déclarait"
        );
        let found = registry.find_by_mod_id(&mod_id, mc, loader).await?;
        let request = Request {
            slug: mod_id.clone(),
            source: None,
            file: None,
            version: None,
            side: Some(side),
            channel: Some(Channel::Beta),
            expected_sha1: None,
            expected_sha512: None,
        };
        match suite_du_rattrapage(pick(found, &request), impasses, mod_id, required_by, side) {
            Rattrapage::Demander(request, reason) => queue.pousser(request, reason, None),
            Rattrapage::Renoncer(manque) => unresolved.push(manque),
        }
    }

    Ok(())
}
