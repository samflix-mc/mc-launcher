//! Ce qu'on tire d'un candidat retenu : ses empreintes, ses dépendances.

use crate::resolve::demande::Request;
use crate::resolve::raison::Reason;
use crate::{Candidate, Channel, DeclaredDep, Origin};

/// Une source qui ne publie pas d'empreinte n'interdit pas de vérifier : celle
/// qu'un passage précédent a figée dans le verrou fait foi.
pub(super) fn completer_empreintes(candidate: &mut Candidate, request: &Request) {
    if candidate.sha1.is_none() {
        candidate.sha1 = request.expected_sha1.clone();
    }
    if candidate.sha512.is_none() {
        candidate.sha512 = request.expected_sha512.clone();
    }
}

/// Met dans la file ce que le build retenu déclare exiger.
pub(super) fn pousser_dependances(
    queue: &mut crate::resolve::file::FileDeResolution,
    deps: Vec<DeclaredDep>,
    source: Origin,
    parent: &str,
    id: &crate::resolve::file::Cle,
) {
    for dep in deps {
        queue.pousser(
            dependance(dep, source),
            Reason::Declared {
                by: parent.to_string(),
            },
            Some(id.clone()),
        );
    }
}

/// La demande que devient une dépendance déclarée.
///
/// Elle se résout dans la source de son parent : un identifiant Modrinth
/// n'existe pas chez CurseForge.
fn dependance(dep: DeclaredDep, source: Origin) -> Request {
    Request {
        slug: dep.project_id,
        source: Some(source),
        file: dep.version_id,
        version: None,
        // Le côté d'une dépendance suit celui de son parent, au minimum ; il
        // sera affiné par le descripteur du jar.
        side: None,
        channel: Some(Channel::Beta),
        expected_sha1: None,
        expected_sha512: None,
    }
}

#[cfg(test)]
#[path = "derivees.test.rs"]
mod tests;
