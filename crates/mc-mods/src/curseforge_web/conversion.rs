//! From a site file to the candidate the resolver works with.

use crate::jar::Side;
use crate::{Candidate, Channel, Origin};

use super::api::WebFile;

/// Loaders that CurseForge names in `gameVersions`.
pub(super) const LOADERS: &[&str] = &["neoforge", "forge", "fabric", "quilt"];

/// Does a file suit this game version and this loader?
///
/// `gameVersions` mixes versions, loaders and sides — e.g.
/// `["1.21", "Client", "1.21.1", "NeoForge", "Server"]`. When no loader
/// appears there, the file is accepted: that's the case for older mods,
/// published before CurseForge started tagging it.
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

/// Download URL from the site, the one its own button uses.
///
/// The CDN URL isn't reconstructed from the id: it's that reconstruction
/// that would bypass an author's refusal to be redistributed. Going through
/// the site's route lets CurseForge decide.
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
        // No digest published by this source: the SHA-1 will be computed on
        // download and then frozen into the lockfile.
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
