//! Retenir un candidat, ou céder la place à celui qui est déjà là.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::resolve::choix::choisir_build;
use crate::resolve::file::{Cle, Demande, FileDeResolution};
use crate::resolve::plan::Installed;
use crate::resolve::raison::{autorite, impasse_implicite};
use crate::resolve::telechargement::side_for;
use crate::resolve::{Options, Registry};

use super::arbitrage::confronter;
use super::derivees::{completer_empreintes, pousser_dependances};

/// Résout une demande et l'inscrit dans la table des retenus.
///
/// Trois issues : le projet est neuf et s'inscrit, il est déjà là et tient sa
/// place, ou il est déjà là et cède à un demandeur plus autoritaire.
#[allow(clippy::too_many_arguments)]
pub(super) async fn retenir(
    registry: &Registry,
    demande: Demande,
    mc: &str,
    loader: &str,
    options: Options,
    chosen: &mut BTreeMap<Cle, Installed>,
    queue: &mut FileDeResolution,
    impasses: &mut BTreeSet<Cle>,
) -> Result<()> {
    let (request, reason) = demande.into_parts();
    let mut candidate = choisir_build(registry, &request, mc, loader).await?;
    completer_empreintes(&mut candidate, &request);

    tracing::debug!(
        slug = %candidate.slug,
        source = candidate.origin.as_str(),
        version = %candidate.version_number,
        fichier = %candidate.file_name,
        raison = %reason.describe(),
        "mod retenu"
    );

    let id = (candidate.origin, candidate.project_id.clone());
    let entrante = autorite(&reason, &request);
    let mut cote = side_for(&request, &candidate);

    if let Some(existing) = chosen.get_mut(&id) {
        let meme_build = existing.candidate.version_id == candidate.version_id;
        if impasse_implicite(&reason, entrante, existing.autorite, meme_build) {
            impasses.insert(id.clone());
        }

        match confronter(existing, &candidate, &reason, entrante, cote, meme_build) {
            None => return Ok(()),
            Some(fusionne) => {
                cote = fusionne;
                // Les dépendances déclarées par le build écarté n'ont plus de
                // demandeur : celles du build qui l'emporte vont être poussées
                // juste après, et peuvent être tout autres.
                queue.oublier_dependances_de(&id);
            }
        }
    }

    let deps = if options.follow_declared {
        candidate.declared_deps.clone()
    } else {
        Vec::new()
    };
    let parent = candidate.name.clone();
    let source = candidate.origin;

    // Écrase l'entrée le cas échéant : le chemin repart vide, donc le bon jar
    // sera téléchargé.
    chosen.insert(
        id.clone(),
        Installed {
            side: cote,
            path: PathBuf::new(),
            provides: BTreeSet::new(),
            requires: Vec::new(),
            autorite: entrante,
            reason,
            candidate,
        },
    );

    pousser_dependances(queue, deps, source, &parent, &id);
    Ok(())
}
