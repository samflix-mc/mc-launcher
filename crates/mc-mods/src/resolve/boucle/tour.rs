//! Un tour : résoudre la file, télécharger, lire les jars, rattraper.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use crate::resolve::file::{Cle, FileDeResolution};
use crate::resolve::inspection::missing_requirements;
use crate::resolve::plan::{Installed, Plan};
use crate::resolve::rattrapage::rattraper;
use crate::resolve::{Options, Registry};

use super::descente::telecharger_et_lire;
use super::retenue::retenir;

/// Faut-il un tour de plus ?
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Suite {
    /// Des exigences ont trouvé un fournisseur : la file n'est pas vide.
    Continuer,
    /// Plus rien ne manque, ou plus rien ne peut être trouvé.
    Termine,
}

/// L'état que le tour fait avancer.
pub(super) struct Etat<'a> {
    pub chosen: &'a mut BTreeMap<Cle, Installed>,
    pub queue: &'a mut FileDeResolution,
    pub impasses: &'a mut BTreeSet<Cle>,
    pub plan: &'a mut Plan,
}

pub(super) async fn un_tour(
    registry: &Registry,
    mc: &str,
    loader: &str,
    options: Options,
    pass: usize,
    etat: &mut Etat<'_>,
) -> Result<Suite> {
    // Résolution en largeur : les dépendances déclarées rejoignent la file.
    while let Some(demande) = etat.queue.suivante() {
        retenir(
            registry,
            demande,
            mc,
            loader,
            options,
            etat.chosen,
            etat.queue,
            etat.impasses,
        )
        .await?;
    }

    telecharger_et_lire(registry, etat.chosen, pass).await?;

    // --- Rattrapage : ce qui manque encore ---
    let missing = missing_requirements(etat.chosen);
    if missing.is_empty() {
        return Ok(Suite::Termine);
    }

    rattraper(
        registry,
        missing,
        mc,
        loader,
        etat.impasses,
        etat.queue,
        &mut etat.plan.unresolved,
    )
    .await?;

    if etat.queue.est_vide() {
        return Ok(Suite::Termine);
    }
    // Un nouveau tour va résoudre la file : les manques déjà consignés
    // pourraient être comblés, on repart d'une liste propre.
    etat.plan.unresolved.clear();
    Ok(Suite::Continuer)
}
