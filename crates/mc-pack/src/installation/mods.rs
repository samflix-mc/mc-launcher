//! Résoudre les mods, puis les poser dans l'instance.

mod deploiement;

use std::path::Path;

use anyhow::Result;

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::{Options, Progress};

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
    log: Progress<'_>,
) -> Result<Pose> {
    let layout = &options.layout;
    let registry = mc_mods::Registry::new(layout.cache().join("mods"))?;
    let requests = if replay {
        let lock = previous.expect("vérifié plus haut");
        tracing::info!(
            builds = lock.mods.len(),
            "Rejeu du verrou : {} builds épinglés",
            lock.mods.len()
        );
        log(&format!(
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
        log("Résolution des mods…");
        manifest.requests()?
    };

    let plan = mc_mods::resolve(&registry, &requests, &manifest.minecraft, "neoforge").await?;
    let added = plan
        .mods
        .iter()
        .filter(|m| m.reason != mc_mods::Reason::Explicit)
        .count();
    tracing::info!(
        total = plan.mods.len(),
        ajoutes = added,
        non_resolus = plan.unresolved.len(),
        "{} mods résolus, dont {added} ajoutés par dépendance",
        plan.mods.len()
    );
    log(&format!(
        "  {} mods, dont {added} ajoutés par résolution des dépendances",
        plan.mods.len()
    ));
    for entry in plan
        .mods
        .iter()
        .filter(|m| matches!(m.reason, mc_mods::Reason::Implicit { .. }))
    {
        log(&format!(
            "  · {} — {}",
            entry.candidate.slug,
            entry.reason.describe()
        ));
    }
    deploiement::deployer(plan, options, manifest, java, neoforge_version, dl, log).await
}
