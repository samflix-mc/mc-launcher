//! Shared factories for this module's test suites.
//!
//! A `Candidate` has eighteen fields and an `Installed` adds six: copying
//! the whole set into every test file would drown out what each one checks,
//! and the slightest field added would demand eight fixes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::jar::{Requirement, Side};
use crate::{Candidate, Channel, Origin};

use super::plan::Installed;
use super::reason::Reason;

/// An ordinary Modrinth candidate: release, redistributable, no dependency.
pub(crate) fn candidate(slug: &str, version: &str) -> Candidate {
    Candidate {
        origin: Origin::Modrinth,
        project_id: format!("{slug}-id"),
        slug: slug.to_string(),
        name: slug.to_string(),
        version_id: format!("{slug}-{version}"),
        version_number: version.to_string(),
        display_name: version.to_string(),
        channel: Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://example.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        published: "2025-01-01".into(),
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: Some(format!("https://modrinth.com/mod/{slug}")),
        redistributable: true,
    }
}

/// A mod already kept, downloaded and inspected: what it supplies, what it
/// requires.
pub(crate) fn installed(slug: &str, provides: &[&str], requires: &[(&str, Side)]) -> Installed {
    let mut candidate = candidate(slug, "1.0");
    candidate.project_id = slug.to_string();
    candidate.version_id = "v".into();
    candidate.page_url = None;

    Installed {
        candidate,
        side: Side::Both,
        reason: Reason::Explicit,
        authority: 4,
        path: PathBuf::from("/cache").join(format!("{slug}.jar")),
        provides: provides.iter().map(ToString::to_string).collect(),
        bundled: std::collections::BTreeSet::new(),
        requires: requires
            .iter()
            .map(|(id, side)| Requirement {
                mod_id: id.to_string(),
                version_range: None,
                side: *side,
            })
            .collect(),
    }
}

/// The same mod, bundling the libraries JarJar reports.
///
/// They join `bundled`, not `provides`: they satisfy dependencies without
/// saying who this mod is.
pub(crate) fn bundling(mut kept: Installed, ids: &[&str]) -> Installed {
    kept.bundled = ids.iter().map(ToString::to_string).collect();
    kept
}

/// The table of kept mods, indexed the way resolution indexes it.
pub(crate) fn map(entries: Vec<Installed>) -> BTreeMap<(Origin, String), Installed> {
    entries
        .into_iter()
        .map(|e| ((e.candidate.origin, e.candidate.project_id.clone()), e))
        .collect()
}
