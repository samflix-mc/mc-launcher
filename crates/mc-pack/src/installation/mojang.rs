//! Les fichiers que Mojang publie : le client, ses bibliothèques, ses assets.

use std::path::Path;

use anyhow::{Context, Result};

use crate::progression::Rapport;

pub(super) async fn poser(
    minecraft: &str,
    shared: &Path,
    dl: &mc_dl::Downloader,
    rapport: &dyn Rapport,
) -> Result<mc_instance::vanilla::Vanilla> {
    rapport.note("Fichiers du jeu…");
    let game = mc_instance::vanilla::install(minecraft, shared, dl)
        .await
        .with_context(|| format!("installation de Minecraft {minecraft}"))?;
    tracing::info!(
        bibliotheques = game.libraries.len(),
        assets_telecharges = game.assets_downloaded,
        index_assets = %game.asset_index_id,
        "Fichiers du jeu en place : {} bibliothèques, {} assets téléchargés",
        game.libraries.len(),
        game.assets_downloaded
    );
    rapport.note(&format!(
        "  {} bibliothèques, {} assets téléchargés",
        game.libraries.len(),
        game.assets_downloaded
    ));

    Ok(game)
}
