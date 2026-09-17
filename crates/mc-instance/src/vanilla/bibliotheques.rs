//! Les bibliothèques qu'une version référence, filtrées par les règles.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::{Path, PathBuf};

use super::PARALLEL;
use super::descripteur::{Artifact, VersionJson};
use super::plateforme::{maven_path, mojang_arch, mojang_os};
use super::regles::allowed;

#[tracing::instrument(name = "bibliothèques", skip_all)]
/// Hors de portée des tests de mutation : cette fonction descend les
/// bibliothèques que le descripteur de version énumère, depuis les serveurs de
/// Mojang. Le choix de celles qui s'appliquent à cette plateforme est vérifié
/// par `regles`, et le téléchargement par mc-dl.
#[mutants::skip]
pub(super) async fn install_libraries(
    version: &VersionJson,
    shared: &Path,
    dl: &Downloader,
) -> Result<Vec<PathBuf>> {
    use futures_util::stream::{self, StreamExt};

    let os = mojang_os();
    let arch = mojang_arch();
    let root = shared.join("libraries");

    let wanted: Vec<(String, Artifact)> = version
        .libraries
        .iter()
        .filter(|lib| allowed(&lib.rules, os, arch))
        .filter_map(|lib| {
            let artifact = lib.downloads.as_ref()?.artifact.as_ref()?;
            let path = artifact.path.clone().or_else(|| maven_path(&lib.name))?;
            Some((
                path,
                Artifact {
                    path: None,
                    sha1: artifact.sha1.clone(),
                    size: artifact.size,
                    url: artifact.url.clone(),
                },
            ))
        })
        .collect();

    // Le descripteur publie la taille de chaque bibliothèque : le lot est donc
    // connu avant d'en demander la première. C'est ce qui permet d'afficher un
    // temps restant plutôt qu'une animation qui tourne dans le vide.
    dl.signaler(mc_dl::Avancement::Lot {
        fichiers: wanted.len(),
        octets: poids(&wanted),
    });

    let results: Vec<Result<PathBuf>> = stream::iter(wanted)
        .map(|(path, artifact)| {
            let dest = root.join(&path);
            async move {
                dl.to_file(
                    &artifact.url,
                    &dest,
                    Check::Full(&Checksum::Sha1(artifact.sha1.clone())),
                )
                .await
                .with_context(|| format!("bibliothèque {path}"))?;
                Ok(dest)
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    results.into_iter().collect()
}

/// Ce que pèse le lot, avant d'en avoir descendu le premier octet.
///
/// Séparée de la boucle pour être vérifiable : une somme fausse ne se voit
/// nulle part ailleurs qu'en regardant une barre de progression se tromper.
fn poids(retenues: &[(String, Artifact)]) -> u64 {
    retenues.iter().map(|(_, artefact)| artefact.size).sum()
}

#[cfg(test)]
#[path = "bibliotheques.test.rs"]
mod tests;
