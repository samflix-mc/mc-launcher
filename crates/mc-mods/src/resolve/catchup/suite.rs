//! What to do with what the search returned.

use std::collections::BTreeSet;

use crate::jar::Side;
use crate::resolve::plan::Unresolved;
use crate::resolve::queue::Key;
use crate::resolve::reason::Reason;
use crate::resolve::request::Request;
use crate::{Candidate, Channel};

/// could supply it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum CatchUp {
    /// A build supplies the `modId`: its request joins the queue.
    Enqueue(Request, Reason),
    /// Nobody will supply it through this path; the gap is recorded and the
    /// install continues — the game will start without it, or not at all,
    /// but that's for the caller to judge, not the resolver.
    GiveUp(Unresolved),
}

/// Decides without asking the network anything, and that's the whole point:
/// the search for a supplier is done, what's left is a judgment on what it
/// returned — a judgment that resolution's termination depends on.
pub(super) fn catch_up_outcome(
    found: Option<Candidate>,
    impasses: &BTreeSet<Key>,
    mod_id: String,
    required_by: String,
    side: Side,
) -> CatchUp {
    // The project already lost this arbitration: proposing it again would
    // replay an identical pass, until [`MAX_PASSES`] runs out.
    let impasse = found
        .as_ref()
        .is_some_and(|c| impasses.contains(&(c.origin, c.project_id.clone())));

    match found {
        Some(candidate) if !impasse => CatchUp::Enqueue(
            Request {
                slug: candidate.project_id.clone(),
                source: Some(candidate.origin),
                file: Some(candidate.version_id.clone()),
                version: None,
                side: Some(side),
                channel: Some(Channel::Beta),
                expected_sha1: None,
                expected_sha512: None,
            },
            Reason::Implicit {
                by: required_by,
                mod_id,
            },
        ),
        other => {
            match &other {
                Some(candidate) => tracing::error!(
                    mod_id = %mod_id,
                    required_by = %required_by,
                    project = %candidate.slug,
                    "\"{mod_id}\", required by {required_by}, could only be supplied by \
                     \"{}\" — whose spot is held by a build a more authoritative request \
                     imposes",
                    candidate.slug
                ),
                None => tracing::error!(
                    mod_id = %mod_id,
                    required_by = %required_by,
                    "\"{mod_id}\", required by {required_by}, is not found on any source"
                ),
            }
            CatchUp::GiveUp(Unresolved {
                mod_id,
                required_by,
                side,
            })
        }
    }
}

#[cfg(test)]
#[path = "suite.test.rs"]
mod tests;
