//! D'un fichier CurseForge au candidat que le résolveur manipule.

use crate::jar::Side;
use crate::{Candidate, Channel, DeclaredDep, Origin};

use super::api::{ApiFile, ApiMod};
use super::{LOADER_FABRIC, LOADER_FORGE, LOADER_NEOFORGE, LOADER_QUILT, RELATION_REQUIRED};

pub(super) fn loader_type(loader: &str) -> u32 {
    match loader.to_ascii_lowercase().as_str() {
        "neoforge" => LOADER_NEOFORGE,
        "forge" => LOADER_FORGE,
        "fabric" => LOADER_FABRIC,
        "quilt" => LOADER_QUILT,
        _ => LOADER_NEOFORGE,
    }
}

/// `releaseType` : 1 = release, 2 = beta, 3 = alpha.
pub(super) fn channel_of(release_type: u32) -> Channel {
    match release_type {
        1 => Channel::Release,
        2 => Channel::Beta,
        _ => Channel::Alpha,
    }
}

pub(super) fn to_candidate(project: &ApiMod, file: ApiFile) -> Option<Candidate> {
    // `algo` : 1 = SHA-1, 2 = MD5. Le SHA-1 est préféré ; les fichiers anciens
    // n'ont parfois qu'un MD5, auquel cas le fichier est téléchargé sans
    // vérification et le lockfile le consigne.
    let sha1 = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.clone());

    let declared = file
        .dependencies
        .iter()
        .filter(|d| d.relation_type == RELATION_REQUIRED)
        .map(|d| DeclaredDep {
            project_id: d.mod_id.to_string(),
            version_id: None,
        })
        .collect();

    Some(Candidate {
        origin: Origin::CurseForge,
        project_id: project.id.to_string(),
        slug: project.slug.clone(),
        name: project.name.clone(),
        version_id: file.id.to_string(),
        version_number: file.display_name.clone(),
        display_name: file.display_name,
        channel: channel_of(file.release_type),
        file_name: file.file_name,
        // Un `downloadUrl` absent traduit le refus de l'auteur d'être
        // redistribué : l'URL n'est pas reconstruite, l'absence est propagée et
        // deviendra un message explicite au moment du téléchargement.
        url: file.download_url.unwrap_or_default(),
        sha1,
        // CurseForge ne publie que du SHA-1 (algo 1) et du MD5 (algo 2).
        sha512: None,
        size: file.file_length,
        published: file.file_date,
        // CurseForge ne publie pas la répartition client/serveur ; le côté sera
        // affiné par le descripteur du jar.
        project_side: Side::Both,
        declared_deps: declared,
        // Conservée pour que le refus de redistribution donne un message
        // actionnable — la page du mod — plutôt qu'un « introuvable ».
        page_url: project.links.website_url.clone(),
        redistributable: project.allow_mod_distribution.unwrap_or(true),
    })
}

#[cfg(test)]
#[path = "conversion.test.rs"]
mod tests;
