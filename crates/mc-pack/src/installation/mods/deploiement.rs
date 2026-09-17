//! Poser les jars dans l'instance, et le serveur si on le demande.

use std::path::Path;

use anyhow::{Context, Result};
use mc_mods::Side;

use crate::Options;
use crate::manifest::Manifest;
use crate::progression::Rapport;

use super::Pose;

#[allow(clippy::too_many_arguments)]
pub(super) async fn deployer(
    plan: mc_mods::Plan,
    options: &Options,
    manifest: &Manifest,
    java: &Path,
    neoforge_version: &str,
    dl: &mc_dl::Downloader,
    rapport: &dyn Rapport,
) -> Result<Pose> {
    let layout = &options.layout;
    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    instance.create()?;
    let server_dir = instance.dir.join("server");

    let client = mc_mods::resolve::deploy(&plan, Side::Client, &instance.mods_dir())?;
    let server = mc_mods::resolve::deploy(&plan, Side::Server, &server_dir.join("mods"))?;
    tracing::info!(
        instance = %instance.name,
        client = client.installed,
        serveur = server.installed,
        retires = client.removed.len() + server.removed.len(),
        "Instance « {} » : {} mods côté client, {} côté serveur",
        instance.name,
        client.installed,
        server.installed
    );

    if options.with_server {
        rapport.note("Serveur NeoForge…");
        mc_instance::neoforge::install_server(
            neoforge_version,
            &server_dir,
            &layout.cache(),
            java,
            dl,
        )
        .await
        .with_context(|| format!("installation du serveur NeoForge {neoforge_version}"))?;
        tracing::info!(
            repertoire = %server_dir.display(),
            "Serveur NeoForge installé dans {}",
            server_dir.display()
        );
    }

    let mut removed = client.removed;
    removed.extend(server.removed);

    Ok(Pose {
        plan,
        instance,
        server_dir,
        client_mods: client.installed,
        server_mods: server.installed,
        removed,
    })
}
