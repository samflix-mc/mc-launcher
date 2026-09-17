//! D'un fichier du site au candidat que le résolveur manipule.

use crate::jar::Side;
use crate::{Candidate, Channel, Origin};

use super::api::WebFile;

/// Chargeurs que CurseForge nomme dans `gameVersions`.
pub(super) const LOADERS: &[&str] = &["neoforge", "forge", "fabric", "quilt"];

/// Un fichier convient-il à cette version du jeu et à ce chargeur ?
///
/// `gameVersions` mélange versions, chargeurs et côtés — p. ex.
/// `["1.21", "Client", "1.21.1", "NeoForge", "Server"]`. Quand aucun chargeur
/// n'y figure, le fichier est accepté : c'est le cas des mods anciens, publiés
/// avant que CurseForge ne l'étiquette.
pub(super) fn compatible(game_versions: &[String], mc: &str, loader: &str) -> bool {
    let lower: Vec<String> = game_versions
        .iter()
        .map(|v| v.to_ascii_lowercase())
        .collect();

    if !lower.iter().any(|v| v == mc) {
        return false;
    }
    let declares_loader = lower.iter().any(|v| LOADERS.contains(&v.as_str()));
    !declares_loader || lower.iter().any(|v| v == loader)
}

pub(super) fn channel_of(release_type: u32) -> Channel {
    match release_type {
        1 => Channel::Release,
        2 => Channel::Beta,
        _ => Channel::Alpha,
    }
}

/// URL de téléchargement du site, celle qu'emprunte son propre bouton.
///
/// L'URL du CDN n'est pas reconstruite à partir de l'identifiant : c'est par
/// cette reconstruction qu'on contournerait le refus d'un auteur d'être
/// redistribué. Passer par la route du site laisse CurseForge décider.
pub(super) fn download_url(web: &str, project_id: u32, file_id: u64) -> String {
    format!("{web}/mods/{project_id}/files/{file_id}/download")
}

pub(super) fn to_candidate(
    web: &str,
    project_id: u32,
    slug: &str,
    name: &str,
    file: WebFile,
) -> Candidate {
    Candidate {
        origin: Origin::CurseForge,
        project_id: project_id.to_string(),
        slug: slug.to_string(),
        name: name.to_string(),
        version_id: file.id.to_string(),
        version_number: file.display_name.clone(),
        display_name: file.display_name,
        channel: channel_of(file.release_type),
        url: download_url(web, project_id, file.id),
        file_name: file.file_name,
        // Aucune empreinte publiée par cette source : le SHA-1 sera calculé au
        // téléchargement puis figé dans le verrou.
        sha1: None,
        sha512: None,
        size: file.file_length,
        published: file.date_created,
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: Some(format!(
            "https://www.curseforge.com/minecraft/mc-mods/{slug}"
        )),
        redistributable: true,
    }
}

#[cfg(test)]
#[path = "conversion.test.rs"]
mod tests;
