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
    //
    // **C'est ici que passe l'essentiel du temps, et c'était muet.** Chaque
    // demande interroge une ou deux API, l'une après l'autre : sur un pack de
    // cinquante mods, une trentaine de secondes pendant lesquelles aucun octet
    // ne descend — donc aucune barre ne bouge, donc l'écran paraît figé. On
    // annonce après CHAQUE demande réglée.
    //
    // Le total est une estimation qui peut grandir, puisqu'une demande résolue
    // peut en faire naître d'autres. Assumé : une barre qui recule un peu se
    // lit, une barre immobile ne se distingue pas d'un plantage.
    let debut = std::time::Instant::now();
    let mut faites = 0;
    registry.annoncer(faites, etat.queue.restantes());
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
        faites += 1;
        registry.annoncer(faites, faites + etat.queue.restantes());
    }

    // **La durée de CETTE boucle, et pas seulement celle de la descente.**
    //
    // Le journal n'annonçait que « N jars téléchargés et analysés en 382 ms »,
    // ce qui laissait croire que l'étape Mods durait une demi-seconde alors
    // qu'elle en prenait trente-six. Les trente-cinq autres sont ici : des
    // appels d'API, l'un après l'autre.
    if faites > 0 {
        tracing::info!(
            tour = pass,
            demandes = faites,
            duree_ms = debut.elapsed().as_millis(),
            "{faites} demandes résolues auprès des API en {} ms (tour {pass})",
            debut.elapsed().as_millis()
        );
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
