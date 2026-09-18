//! The files Mojang publishes: the client, its libraries, its assets.

use std::path::Path;

use anyhow::{Context, Result};

use crate::progress::Report;

pub(super) async fn place(
    minecraft: &str,
    shared: &Path,
    dl: &mc_dl::Downloader,
    report: &dyn Report,
) -> Result<mc_instance::vanilla::Vanilla> {
    report.note("Game files…");
    let game = mc_instance::vanilla::install(minecraft, shared, dl)
        .await
        .with_context(|| format!("installing Minecraft {minecraft}"))?;
    tracing::info!(
        libraries = game.libraries.len(),
        assets_downloaded = game.assets_downloaded,
        asset_index = %game.asset_index_id,
        "Game files in place: {} libraries, {} assets downloaded",
        game.libraries.len(),
        game.assets_downloaded
    );
    report.note(&format!(
        "  {} libraries, {} assets downloaded",
        game.libraries.len(),
        game.assets_downloaded
    ));

    Ok(game)
}
