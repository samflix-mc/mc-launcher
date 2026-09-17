//! Les milliers d'objets qu'une version référence.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::Path;

use super::descripteur::{AssetIndex, AssetIndexRef, AssetObject};
use super::{PARALLEL, RESOURCES};

#[tracing::instrument(name = "assets", skip_all, fields(index = %index.id))]
/// Hors de portée des tests de mutation : cette fonction descend les milliers
/// d'objets que Mojang publie, par une adresse écrite dans ce module. Ce
/// qu'elle en compte se vérifie — voir [`compter_les_telechargements`] —, et
/// le téléchargement lui-même est vérifié chez mc-dl.
#[mutants::skip]
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

    // L'index est lu avant de commencer : c'est le seul moment où l'on sait ce
    // que pèse l'étape la plus longue de l'installation. Sans cette annonce,
    // aucun temps restant n'est calculable — les octets arriveraient sans
    // qu'on sache jamais combien il en manque.
    let attendus: Vec<_> = parsed.objects.into_values().collect();
    dl.signaler(mc_dl::Avancement::Lot {
        fichiers: attendus.len(),
        octets: poids(&attendus),
    });

    let results: Vec<Result<mc_dl::Fetched>> = stream::iter(attendus)
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

    let total = results.len();
    let downloaded = compter_les_telechargements(results)?;
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

/// Ce que pèse le lot, avant d'en avoir descendu le premier octet.
///
/// Séparée de la boucle pour être vérifiable : une somme fausse ne se voit
/// nulle part ailleurs qu'en regardant une barre de progression se tromper.
fn poids(objets: &[AssetObject]) -> u64 {
    objets.iter().map(|objet| objet.size).sum()
}

/// Combien d'objets ont été réellement téléchargés, sur ceux qu'on a demandés.
///
/// La première erreur rencontrée arrête tout : un asset manquant fait une
/// texture absente, pas un jeu qui refuse de démarrer, et l'on préfère le
/// savoir tout de suite. Le compte, lui, sépare une première installation —
/// des milliers d'objets — d'une seconde, où tout est déjà là : c'est la seule
/// explication qu'on ait de l'attente.
fn compter_les_telechargements(resultats: Vec<Result<mc_dl::Fetched>>) -> Result<usize> {
    let mut telecharges = 0;
    for resultat in resultats {
        if resultat? == mc_dl::Fetched::Downloaded {
            telecharges += 1;
        }
    }
    Ok(telecharges)
}

#[cfg(test)]
#[path = "assets.test.rs"]
mod tests;
