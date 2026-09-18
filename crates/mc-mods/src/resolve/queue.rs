//! The resolution queue: what is left to resolve, and in what order.

use crate::Origin;

use super::reason::Reason;
use super::request::Request;

/// Uniqueness key for a project: a mod can only be present once.
pub(super) type Key = (Origin, String);

/// A request awaiting resolution.
pub(super) struct PendingRequest {
    request: Request,
    reason: Reason,
    /// The build that pushed this request, when it follows from another one.
    ///
    /// The key, not the title carried by [`Reason::Declared`]: "Jade" is
    /// published under the same name on Modrinth and on CurseForge, and both
    /// projects coexist in `chosen` until the final deduplication. Removing
    /// the dependencies of a build discarded by its title would therefore
    /// also remove those of its namesake, which nothing else still required.
    parent: Option<Key>,
}

impl PendingRequest {
    /// What is needed to resolve it. The parent only matters to the queue,
    /// which keeps it to remove the dependencies of a discarded build.
    pub(super) fn into_parts(self) -> (Request, Reason) {
        (self.request, self.reason)
    }
}

/// Resolution queue: the manifest first, then the dependencies.
///
/// A single queue was enough as long as only the result mattered. But it
/// emptied like a stack: the dependency of a request already popped would
/// jump ahead of the remaining requests. A mod both pinned by the lock and a
/// dependency of another one was then resolved without its pin, and the
/// pinned request would later land on a key already taken.
#[derive(Default)]
pub(super) struct ResolutionQueue {
    manifest: Vec<PendingRequest>,
    derived: Vec<PendingRequest>,
}

impl ResolutionQueue {
    pub(super) fn push(&mut self, request: Request, reason: Reason, parent: Option<Key>) {
        let entry = PendingRequest {
            request,
            reason,
            parent,
        };
        match entry.reason {
            Reason::Explicit => self.manifest.push(entry),
            _ => self.derived.push(entry),
        }
    }

    /// Pops depth-first, but never a dependency while the manifest isn't
    /// fully processed yet.
    pub(super) fn next(&mut self) -> Option<PendingRequest> {
        self.manifest.pop().or_else(|| self.derived.pop())
    }

    /// Removes from the queue the dependencies a build had declared.
    ///
    /// Called when a build replaces another one: the discarded build's
    /// dependencies no longer have a requester. Leaving them would install
    /// jars that nothing asks for anymore — and the lock would record them
    /// as dependencies of a version that isn't the one kept.
    ///
    /// Only catches what is still in the queue. A dependency of the old
    /// build already resolved in a previous pass stays there: removing it
    /// would require knowing who else relies on it, i.e. keeping the
    /// reverse graph. The gap is bounded — one library jar too many, which
    /// NeoForge loads without complaint — and nowhere near as bad as the
    /// defect on the other side of it, which was installing the wrong
    /// pinned build.
    pub(super) fn forget_dependencies_of(&mut self, parent: &Key) {
        self.derived
            .retain(|entry| entry.parent.as_ref() != Some(parent));
    }

    /// How many requests are still waiting their turn.
    ///
    /// Only used to announce progress, nothing else: the total it gives
    /// MOVES, since resolving one request can spawn others. That's
    /// accepted — a bar that steps back a little beats a window frozen for
    /// the thirty-six seconds the API polling can take.
    pub(super) fn remaining(&self) -> usize {
        self.manifest.len() + self.derived.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.manifest.is_empty() && self.derived.is_empty()
    }
}

#[cfg(test)]
#[path = "queue.test.rs"]
mod tests;

#[cfg(test)]
#[path = "queue.forgotten.test.rs"]
mod tests_forgotten;
