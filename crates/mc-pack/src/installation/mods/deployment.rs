//! Place the jars in the instance, and the server if requested.

use std::path::Path;

use anyhow::{Context, Result};
use mc_mods::Side;

use crate::Options;
use crate::manifest::Manifest;
use crate::progress::Report;

use super::Placement;

#[allow(clippy::too_many_arguments)]
pub(super) async fn deploy(
    plan: mc_mods::Plan,
    options: &Options,
    manifest: &Manifest,
    java: &Path,
    neoforge_version: &str,
    dl: &mc_dl::Downloader,
    report: &dyn Report,
) -> Result<Placement> {
    let layout = &options.layout;
    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    instance.create()?;
    let server_dir = instance.dir.join("server");

    let client = mc_mods::resolve::deploy(&plan, Side::Client, &instance.mods_dir())?;
    let server = mc_mods::resolve::deploy(&plan, Side::Server, &server_dir.join("mods"))?;
    tracing::info!(
        instance = %instance.name,
        client = client.installed,
        server = server.installed,
        removed = client.removed.len() + server.removed.len(),
        "Instance \"{}\" : {} client-side mods, {} server-side",
        instance.name,
        client.installed,
        server.installed
    );

    if options.with_server {
        report.note("NeoForge server…");
        mc_instance::neoforge::install_server(
            neoforge_version,
            &server_dir,
            &layout.cache(),
            java,
            dl,
        )
        .await
        .with_context(|| format!("installing NeoForge server {neoforge_version}"))?;
        tracing::info!(
            directory = %server_dir.display(),
            "NeoForge server installed in {}",
            server_dir.display()
        );
    }

    let mut removed = client.removed;
    removed.extend(server.removed);

    Ok(Placement {
        plan,
        instance,
        server_dir,
        client_mods: client.installed,
        server_mods: server.installed,
        removed,
    })
}
