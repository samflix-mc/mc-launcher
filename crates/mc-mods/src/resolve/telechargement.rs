//! Ce qui descend sur le disque, et à quelle allure.

use anyhow::{Context, Result};
use std::collections::BTreeMap;

use crate::jar::Side;
use crate::{Candidate, Origin};
use std::path::PathBuf;

use super::demande::Request;
use super::plan::Installed;
use super::Registry;
use super::PARALLEL_DOWNLOADS;


/// Côté retenu : le manifeste prime, sinon les métadonnées du projet.
pub(super) fn side_for(request: &Request, candidate: &Candidate) -> Side {
    request.side.unwrap_or(candidate.project_side)
}

/// Télécharge ce qui n'a pas encore de chemin, en parallèle borné.
pub(super) async fn download_all(
    registry: &Registry,
    chosen: &mut BTreeMap<(Origin, String), Installed>,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};

    struct Job {
        key: (Origin, String),
        url: String,
        file_name: String,
        sum: Option<mc_dl::Checksum>,
        size: u64,
    }

    let todo: Vec<Job> = chosen
        .iter()
        .filter(|(_, m)| m.path.as_os_str().is_empty())
        .map(|(key, m)| Job {
            key: key.clone(),
            url: m.candidate.url.clone(),
            file_name: m.candidate.file_name.clone(),
            sum: m.candidate.checksum(),
            size: m.candidate.size,
        })
        .collect();

    let results: Vec<Result<((Origin, String), PathBuf)>> = stream::iter(todo)
        .map(|job| {
            let dl = registry.dl.clone();
            // Le cache est indexé par source et par projet : deux mods
            // différents publient parfois un jar au même nom.
            let dest = registry
                .cache
                .join(job.key.0.as_str())
                .join(&job.key.1)
                .join(&job.file_name);
            async move {
                // Un jar est du code exécuté : son empreinte est recontrôlée à
                // chaque passage, pas seulement à l'écriture. À défaut
                // d'empreinte, la taille annoncée est le seul garde-fou.
                let check = match (&job.sum, job.size) {
                    (Some(sum), _) => mc_dl::Check::Full(sum),
                    (None, 0) => mc_dl::Check::Presence,
                    (None, size) => mc_dl::Check::Size(size),
                };
                dl.to_file(&job.url, &dest, check)
                    .await
                    .with_context(|| format!("téléchargement de {}", job.file_name))?;
                Ok((job.key, dest))
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOADS)
        .collect()
        .await;

    for result in results {
        let (key, path) = result?;
        if let Some(entry) = chosen.get_mut(&key) {
            // Source sans empreinte : on calcule la nôtre. Elle part dans le
            // verrou, et les installations suivantes seront vérifiées comme
            // toutes les autres.
            if entry.candidate.checksum().is_none() {
                entry.candidate.sha512 = mc_dl::sha512_of_file(&path).ok();
            }
            entry.path = path;
        }
    }
    Ok(())
}
