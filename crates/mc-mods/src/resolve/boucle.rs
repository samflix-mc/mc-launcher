//! Les tours de résolution, jusqu'à ce que plus rien ne manque.

mod arbitrage;
mod derivees;
mod descente;
mod retenue;
mod tour;

use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

use super::demande::Request;
use super::doublons::deduplicate_by_mod_id;
use super::file::{Cle, FileDeResolution};
use super::plan::{Installed, Plan};
use super::raison::Reason;
use super::{MAX_PASSES, Options, Registry};

use tour::{Etat, Suite, un_tour};

/// Résout, télécharge et vérifie l'ensemble du pack.
pub async fn resolve(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
) -> Result<Plan> {
    resolve_with(registry, requests, mc, loader, Options::default()).await
}

#[tracing::instrument(
    name = "résolution",
    skip(registry, requests, options),
    fields(demandes = requests.len(), mc, loader)
)]
pub async fn resolve_with(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
    options: Options,
) -> Result<Plan> {
    // Clé d'unicité : un projet ne peut être présent qu'une fois. Deux versions
    // du même mod dans `mods` font échouer NeoForge au chargement.
    let mut chosen: BTreeMap<Cle, Installed> = BTreeMap::new();
    let mut plan = Plan::default();
    // Les projets qu'une exigence implicite a tenté de prendre, sans l'emporter.
    // Voir [`super::raison::impasse_implicite`] : sans cette mémoire, le
    // rattrapage repousse indéfiniment une demande qui reperd le même arbitrage.
    let mut impasses: BTreeSet<Cle> = BTreeSet::new();

    // Tour 1 : ce que le manifeste demande, et ce que les API déclarent.
    let mut queue = FileDeResolution::default();
    for request in requests {
        queue.pousser(request.clone(), Reason::Explicit, None);
    }

    let mut etat = Etat {
        chosen: &mut chosen,
        queue: &mut queue,
        impasses: &mut impasses,
        plan: &mut plan,
    };

    for pass in 1..=MAX_PASSES {
        if un_tour(registry, mc, loader, options, pass, &mut etat).await? == Suite::Termine {
            break;
        }
        if pass == MAX_PASSES {
            bail!(
                "la résolution ne se stabilise pas après {MAX_PASSES} tours — \
                 dépendances circulaires ou introuvables"
            );
        }
    }

    deduplicate_by_mod_id(&mut chosen);
    plan.mods = chosen.into_values().collect();
    plan.mods
        .sort_by(|a, b| a.candidate.slug.cmp(&b.candidate.slug));
    Ok(plan)
}

#[cfg(test)]
#[path = "boucle.test.rs"]
mod tests;
