//! From an API version to the candidate the resolver works with.

use crate::jar::Side;
use crate::{Candidate, Channel, DeclaredDep, Origin};

use super::api::{ApiVersion, Project};

fn side_of(project: &Project) -> Side {
    let client = project.client_side != "unsupported";
    let server = project.server_side != "unsupported";
    match (client, server) {
        (true, false) => Side::Client,
        (false, true) => Side::Server,
        _ => Side::Both,
    }
}

pub(super) fn to_candidate(project: &Project, version: ApiVersion) -> Option<Candidate> {
    // A version sometimes carries several files (sources, variants); the
    // "primary" file is the one the launcher must install.
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())?;

    let declared = version
        .dependencies
        .iter()
        .filter(|d| d.dependency_type == "required")
        .filter_map(|d| {
            Some(DeclaredDep {
                project_id: d.project_id.clone()?,
                version_id: d.version_id.clone(),
            })
        })
        .collect();

    Some(Candidate {
        origin: Origin::Modrinth,
        project_id: project.id.clone(),
        slug: project.slug.clone(),
        name: project.title.clone(),
        version_id: version.id,
        version_number: version.version_number,
        display_name: version.name,
        channel: Channel::parse(&version.version_type),
        file_name: file.filename.clone(),
        url: file.url.clone(),
        sha1: file.hashes.sha1.clone(),
        sha512: file.hashes.sha512.clone(),
        size: file.size,
        published: version.date_published,
        project_side: side_of(project),
        declared_deps: declared,
        page_url: Some(format!("https://modrinth.com/mod/{}", project.slug)),
        redistributable: true,
    })
}

#[cfg(test)]
#[path = "conversion.test.rs"]
mod tests;
