//! Le build à retenir pour une demande, ou l'explication de son absence.

use anyhow::{Result, bail};

use super::Registry;
use super::demande::Request;
use super::registre::filtre::pick;
use crate::Candidate;

/// décrites, et dont aucune ne regarde l'état de la résolution.
pub(super) async fn choisir_build(
    registry: &Registry,
    request: &Request,
    mc: &str,
    loader: &str,
) -> Result<Candidate> {
    if let Some(pinned) = registry.pinned(request, mc, loader).await? {
        return Ok(pinned);
    }

    let found = registry
        .candidates(&request.slug, request.source, mc, loader)
        .await?;
    trancher(found, request, mc, loader, registry.has_curseforge())
}

/// Ce qu'on retient d'une liste de candidats, ou pourquoi on ne retient rien.
///
/// Séparé de [`choisir_build`] pour une raison simple : le jugement ne dépend
/// que de ce que la source a répondu, jamais du réseau. Mêlé à l'appel, il
/// était intestable, et ce sont pourtant ces trois messages que le joueur lira
/// le jour où son pack ne s'installe pas.
pub(super) fn trancher(
    found: Vec<Candidate>,
    request: &Request,
    mc: &str,
    loader: &str,
    has_curseforge: bool,
) -> Result<Candidate> {
    let had_candidates = !found.is_empty();

    let candidate = match pick(found, request) {
        Some(c) => c,
        // Le projet existe, mais rien n'y correspond : dire quoi, sans quoi le
        // message envoie chercher un mod absent alors qu'il est sous les yeux.
        None if had_candidates => bail!(
            "{} : aucune version ne correspond{}",
            request.slug,
            match (&request.version, request.channel) {
                (Some(v), _) => format!(" à la version demandée « {v} »"),
                (None, Some(ch)) => format!(" au canal {} ou plus stable", ch.as_str()),
                _ => " au canal release".to_string(),
            }
        ),
        None => {
            let hint = if has_curseforge {
                ""
            } else {
                " (aucune clé CurseForge configurée : seul Modrinth a été consulté)"
            };
            bail!(
                "{} : introuvable pour Minecraft {mc} / {loader}{hint}",
                request.slug
            );
        }
    };

    if !candidate.redistributable || candidate.url.is_empty() {
        bail!(
            "{} : l'auteur a désactivé le téléchargement par un launcher tiers. \
             Récupérer le fichier sur {} et le déposer dans le dossier des apports manuels.",
            candidate.slug,
            candidate.page_url.as_deref().unwrap_or("la page du mod")
        );
    }

    Ok(candidate)
}

#[cfg(test)]
#[path = "choix.test.rs"]
mod tests;

#[cfg(test)]
#[path = "choix.refus.test.rs"]
mod tests_refus;
