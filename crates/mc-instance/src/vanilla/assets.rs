//! Les milliers d'objets qu'une version référence.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::Path;

use super::descripteur::{AssetIndex, AssetIndexRef};
use super::{PARALLEL, RESOURCES};

#[tracing::instrument(name = "assets", skip_all, fields(index = %index.id))]
pub(super) async fn install_assets(
    index: &AssetIndexRef,
    shared: &Path,
    dl: &Downloader,
) -> Result<usize> {
    use futures_util::stream::{self, StreamExt};

    let index_path = shared
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", index.id));
    dl.to_file(
        &index.url,
        &index_path,
        Check::Full(&Checksum::Sha1(index.sha1.clone())),
    )
    .await
    .context("téléchargement de l'index des assets")?;

    let parsed: AssetIndex = serde_json::from_slice(&tokio::fs::read(&index_path).await?)
        .context("index des assets illisible")?;
    let objects = shared.join("assets").join("objects");

    let results: Vec<Result<mc_dl::Fetched>> = stream::iter(parsed.objects.into_values())
        .map(|object| {
            // Les objets sont adressés par leur empreinte : deux versions du
            // jeu partagent tout ce qui n'a pas changé.
            let prefix = &object.hash[..2];
            let dest = objects.join(prefix).join(&object.hash);
            let url = format!("{RESOURCES}/{prefix}/{}", object.hash);
            async move {
                let sum = Checksum::Sha1(object.hash.clone());
                dl.to_file(
                    &url,
                    &dest,
                    Check::Quick {
                        sum: &sum,
                        size: object.size,
                    },
                )
                .await
                .with_context(|| format!("asset {}", object.hash))
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    let mut downloaded = 0;
    let total = results.len();
    for result in results {
        if result? == mc_dl::Fetched::Downloaded {
            downloaded += 1;
        }
    }
    // L'étape la plus longue d'une première installation, et la plus muette
    // d'une seconde : dire combien d'objets ont été passés explique pourquoi.
    tracing::info!(
        total,
        telecharges = downloaded,
        deja_presents = total - downloaded,
        "{downloaded} assets téléchargés sur {total} ({} déjà présents)",
        total - downloaded
    );
    Ok(downloaded)
}
