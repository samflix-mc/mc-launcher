//! What the pack requests, resolved against the sources.

use anyhow::Result;

use mc_pack::manifest::Manifest;

/// Returns the plan picked and the NeoForge version that comes with it.
/// Out of scope for mutation testing: this function queries Modrinth and
/// CurseForge to resolve the whole pack. What it decides is tested over at
/// `mc_mods::resolve`, which has a test server; what's left here is
/// assembling the registry and the report.
#[mutants::skip]
pub(super) async fn resolve(
    manifest: &Manifest,
    options: &mc_pack::Options,
) -> Result<(mc_mods::Plan, String)> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    tracing::info!(
        pack = %manifest.name,
        minecraft = %manifest.minecraft,
        requests = manifest.mods.len(),
        "Pack \"{}\": Minecraft {}, {} mods requested",
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
        pinned = !manifest.loader.is_latest(),
        "NeoForge {} selected ({})",
        neoforge_version,
        if manifest.loader.is_latest() {
            "latest published version"
        } else {
            "pinned by the manifest"
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

    let added = plan
        .mods
        .iter()
        .filter(|m| m.reason != mc_mods::Reason::Explicit)
        .count();
    tracing::info!(
        total = plan.mods.len(),
        added,
        unresolved = plan.unresolved.len(),
        "{} mods resolved, {added} added as dependencies",
        plan.mods.len()
    );

    Ok((plan, neoforge_version))
}
