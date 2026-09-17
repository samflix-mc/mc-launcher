//! Fabriques partagées par les suites de ce module.
//!
//! Un `Candidate` a dix-huit champs et un `Installed` en ajoute six : recopier
//! l'ensemble dans chaque fichier de tests noierait ce que chacun vérifie, et
//! le moindre champ ajouté demanderait huit corrections.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::jar::{Requirement, Side};
use crate::{Candidate, Channel, Origin};

use super::plan::Installed;
use super::raison::Reason;

/// Un candidat Modrinth ordinaire : release, redistribuable, sans dépendance.
pub(crate) fn candidat(slug: &str, version: &str) -> Candidate {
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
        url: format!("https://exemple.invalid/{slug}.jar"),
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

/// Un mod déjà retenu, téléchargé et analysé : ce qu'il fournit, ce qu'il
/// exige.
pub(crate) fn installed(slug: &str, provides: &[&str], requires: &[(&str, Side)]) -> Installed {
    let mut candidate = candidat(slug, "1.0");
    candidate.project_id = slug.to_string();
    candidate.version_id = "v".into();
    candidate.page_url = None;

    Installed {
        candidate,
        side: Side::Both,
        reason: Reason::Explicit,
        autorite: 4,
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

/// Le même mod, qui embarque les bibliothèques données par JarJar.
///
/// Elles rejoignent `bundled` et non `provides` : elles satisfont des
/// dépendances sans dire qui est ce mod.
pub(crate) fn embarquant(mut retenu: Installed, ids: &[&str]) -> Installed {
    retenu.bundled = ids.iter().map(ToString::to_string).collect();
    retenu
}

/// La table des retenus, indexée comme la résolution l'indexe.
pub(crate) fn map(entries: Vec<Installed>) -> BTreeMap<(Origin, String), Installed> {
    entries
        .into_iter()
        .map(|e| ((e.candidate.origin, e.candidate.project_id.clone()), e))
        .collect()
}
