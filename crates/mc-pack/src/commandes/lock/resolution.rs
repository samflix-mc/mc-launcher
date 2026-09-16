//! Ce que le pack demande, résolu contre les sources.

use anyhow::Result;

use mc_pack::manifest::Manifest;

/// Rend le plan retenu et la version de NeoForge qui l'accompagne.
pub(super) async fn resoudre(
    manifest: &Manifest,
    options: &mc_pack::Options,
) -> Result<(mc_mods::Plan, String)> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    tracing::info!(
        pack = %manifest.name,
        minecraft = %manifest.minecraft,
        demandes = manifest.mods.len(),
        "Pack « {} » : Minecraft {}, {} mods demandés",
        manifest.name,
        manifest.minecraft,
        manifest.mods.len()
    );

    let neoforge_version = if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, &dl).await?
    } else {
        manifest.loader.version.clone()
    };
    tracing::info!(
        neoforge = %neoforge_version,
        epingle = !manifest.loader.is_latest(),
        "NeoForge {} retenu ({})",
        neoforge_version,
        if manifest.loader.is_latest() {
            "dernière version publiée"
        } else {
            "épinglé par le manifeste"
        }
    );

    let registry = mc_mods::Registry::new(options.layout.cache().join("mods"))?;
    let plan = mc_mods::resolve(
        &registry,
        &manifest.requests()?,
        &manifest.minecraft,
        "neoforge",
    )
    .await?;

    let ajoutes = plan
        .mods
        .iter()
        .filter(|m| m.reason != mc_mods::Reason::Explicit)
        .count();
    tracing::info!(
        total = plan.mods.len(),
        ajoutes,
        non_resolus = plan.unresolved.len(),
        "{} mods résolus, dont {ajoutes} ajoutés par dépendance",
        plan.mods.len()
    );

    Ok((plan, neoforge_version))
}
