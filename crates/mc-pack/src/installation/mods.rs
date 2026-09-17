//! Résoudre les mods, puis les poser dans l'instance.

mod deploiement;

use std::path::Path;

use anyhow::Result;

use std::sync::Arc;

use crate::Options;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::progression::Rapport;

/// Ce que l'étape « mods » laisse derrière elle.
pub(super) struct Pose {
    pub plan: mc_mods::Plan,
    pub instance: mc_instance::Instance,
    pub server_dir: std::path::PathBuf,
    pub client_mods: usize,
    pub server_mods: usize,
    pub removed: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn poser(
    manifest: &Manifest,
    previous: Option<&Lockfile>,
    options: &Options,
    replay: bool,
    java: &Path,
    neoforge_version: &str,
    dl: &mc_dl::Downloader,
    rapport: &Arc<dyn Rapport>,
) -> Result<Pose> {
    let layout = &options.layout;
    // Le registre monte son propre client HTTP : sans cet observateur-là, les
    // mods seraient la seule étape à descendre en silence, et c'est la plus
    // longue après les assets.
    let registry =
        mc_mods::Registry::observee(layout.cache().join("mods"), super::observateur(rapport))?;
    let requests = if replay {
        let lock = previous.expect("vérifié plus haut");
        tracing::info!(
            builds = lock.mods.len(),
            "Rejeu du verrou : {} builds épinglés",
            lock.mods.len()
        );
        rapport.note(&format!(
            "Mods : {} builds rejoués depuis le verrou",
            lock.mods.len()
        ));
        lock.requests()
    } else {
        tracing::info!(
            demandes = manifest.mods.len(),
            "Résolution de {} mods demandés",
            manifest.mods.len()
        );
        rapport.note("Résolution des mods…");
        manifest.requests()?
    };

    let plan = mc_mods::resolve(&registry, &requests, &manifest.minecraft, "neoforge").await?;
    let added = ajoutes_par_dependance(plan.mods.iter().map(|m| &m.reason));
    tracing::info!(
        total = plan.mods.len(),
        ajoutes = added,
        non_resolus = plan.unresolved.len(),
        "{} mods résolus, dont {added} ajoutés par dépendance",
        plan.mods.len()
    );
    rapport.note(&format!(
        "  {} mods, dont {added} ajoutés par résolution des dépendances",
        plan.mods.len()
    ));
    for entry in plan
        .mods
        .iter()
        .filter(|m| matches!(m.reason, mc_mods::Reason::Implicit { .. }))
    {
        rapport.note(&format!(
            "  · {} — {}",
            entry.candidate.slug,
            entry.reason.describe()
        ));
    }
    deploiement::deployer(
        plan,
        options,
        manifest,
        java,
        neoforge_version,
        dl,
        rapport.as_ref(),
    )
    .await
}

/// Combien de mods le pack a gagnés sans que le manifeste les demande.
///
/// C'est le chiffre qui explique qu'un manifeste de trente lignes installe
/// cent mods : le reste vient des dépendances. Compter les autres — ceux que
/// le manifeste nomme — annoncerait le contraire, et ferait croire à une
/// résolution qui n'a rien trouvé.
fn ajoutes_par_dependance<'a>(raisons: impl Iterator<Item = &'a mc_mods::Reason>) -> usize {
    raisons
        .filter(|reason| **reason != mc_mods::Reason::Explicit)
        .count()
}

#[cfg(test)]
#[path = "mods.test.rs"]
mod tests;
