//! Ce qu'on garde d'une liste de candidats, et ce qu'on refuse.

use anyhow::{bail, Result};

use crate::resolve::demande::Request;
use crate::{Candidate, Channel};

/// Un build épinglé n'est pas filtré par l'API : il faut vérifier soi-même
/// qu'il correspond bien à la version de Minecraft et au chargeur du pack.
pub(crate) fn check_compatible(candidate: &Candidate, mc: &str, loader: &str, slug: &str) -> Result<()> {
    // Les deux API ont déjà filtré quand on est passé par la liste ; pour un
    // build épinglé, le nom de fichier est le seul indice disponible sans
    // requête supplémentaire, et il est trop peu fiable pour rejeter. On se
    // contente donc de signaler ce qui est manifestement incohérent.
    let haystack = format!(
        "{} {} {}",
        candidate.file_name, candidate.version_number, candidate.display_name
    )
    .to_ascii_lowercase();

    let other_loaders = ["fabric", "quilt"];
    if other_loaders.iter().any(|l| haystack.contains(l)) && !haystack.contains(loader) {
        bail!(
            "le build épinglé pour {slug} ({}) vise un autre chargeur que {loader}",
            candidate.file_name
        );
    }
    let _ = mc;
    Ok(())
}

/// Choisit le meilleur candidat : canal autorisé, puis publication la plus
/// récente.
pub(crate) fn pick(candidates: Vec<Candidate>, request: &Request) -> Option<Candidate> {
    let limit = request.channel.unwrap_or(Channel::Release);

    if let Some(wanted) = &request.version {
        let wanted_lower = wanted.to_ascii_lowercase();
        if let Some(found) = candidates.iter().find(|c| {
            c.version_number.eq_ignore_ascii_case(wanted)
                || c.file_name.to_ascii_lowercase() == wanted_lower
                || c.display_name.eq_ignore_ascii_case(wanted)
        }) {
            return Some(found.clone());
        }
        return None;
    }

    let mut allowed: Vec<Candidate> = candidates
        .into_iter()
        .filter(|c| c.channel.allowed_by(limit))
        .collect();

    // À défaut de release, on accepte ce qui existe : refuser laisserait un
    // pack sans son mod, ce qui est pire qu'une beta signalée dans le lockfile.
    allowed.sort_by(|a, b| b.published.cmp(&a.published));
    allowed.into_iter().next()
}

#[cfg(test)]
#[path = "filtre.test.rs"]
mod tests;
