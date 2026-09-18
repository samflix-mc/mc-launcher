//! What we pull from a kept candidate: its digests, its dependencies.

use crate::resolve::reason::Reason;
use crate::resolve::request::Request;
use crate::{Candidate, Channel, DeclaredDep, Origin};

/// A source that doesn't publish a digest doesn't stop verification: the one
/// a previous pass fixed in the lock makes it good enough.
pub(super) fn complete_digests(candidate: &mut Candidate, request: &Request) {
    if candidate.sha1.is_none() {
        candidate.sha1 = request.expected_sha1.clone();
    }
    if candidate.sha512.is_none() {
        candidate.sha512 = request.expected_sha512.clone();
    }
}

/// Puts into the queue what the kept build declares it requires.
pub(super) fn push_dependencies(
    queue: &mut crate::resolve::queue::ResolutionQueue,
    deps: Vec<DeclaredDep>,
    source: Origin,
    parent: &str,
    id: &crate::resolve::queue::Key,
) {
    for dep in deps {
        queue.push(
            dependency(dep, source),
            Reason::Declared {
                by: parent.to_string(),
            },
            Some(id.clone()),
        );
    }
}

/// What a declared dependency becomes as a request.
///
/// It resolves within its parent's source: a Modrinth identifier doesn't
/// exist on CurseForge.
fn dependency(dep: DeclaredDep, source: Origin) -> Request {
    Request {
        slug: dep.project_id,
        source: Some(source),
        file: dep.version_id,
        version: None,
        // A dependency's side follows its parent's, at minimum; it will be
        // refined by the jar's descriptor.
        side: None,
        channel: Some(Channel::Beta),
        expected_sha1: None,
        expected_sha512: None,
    }
}

#[cfg(test)]
#[path = "derived.test.rs"]
mod tests;
