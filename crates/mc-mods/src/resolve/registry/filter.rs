//! What we keep from a list of candidates, and what we refuse.

use anyhow::{Result, bail};

use crate::resolve::request::Request;
use crate::{Candidate, Channel};

/// A pinned build isn't filtered by the API: you have to check yourself that
/// it matches the pack's Minecraft version and loader.
pub(crate) fn check_compatible(
    candidate: &Candidate,
    mc: &str,
    loader: &str,
    slug: &str,
) -> Result<()> {
    // Both APIs already filtered when we went through the list; for a pinned
    // build, the file name is the only clue available without an extra
    // request, and it's too unreliable to reject on. So we only flag what's
    // clearly inconsistent.
    let haystack = format!(
        "{} {} {}",
        candidate.file_name, candidate.version_number, candidate.display_name
    )
    .to_ascii_lowercase();

    let other_loaders = ["fabric", "quilt"];
    if other_loaders.iter().any(|l| haystack.contains(l)) && !haystack.contains(loader) {
        bail!(
            "the pinned build for {slug} ({}) targets a different loader than {loader}",
            candidate.file_name
        );
    }
    let _ = mc;
    Ok(())
}

/// Picks the best candidate: allowed channel, then most recent publication.
pub(crate) fn pick(candidates: Vec<Candidate>, request: &Request) -> Option<Candidate> {
    let limit = request.channel.unwrap_or(Channel::Release);

    if let Some(wanted) = &request.version {
        let wanted_lower = wanted.to_ascii_lowercase();
        if let Some(found) = candidates.iter().find(|c| {
            c.version_number.eq_ignore_ascii_case(wanted)
                || c.file_name.to_ascii_lowercase() == wanted_lower
                || c.display_name.eq_ignore_ascii_case(wanted)
        }) {
            return Some(found.clone());
        }
        return None;
    }

    let mut allowed: Vec<Candidate> = candidates
        .into_iter()
        .filter(|c| c.channel.allowed_by(limit))
        .collect();

    // Absent a release, we accept what exists: refusing would leave a pack
    // without its mod, which is worse than a beta flagged in the lockfile.
    allowed.sort_by(|a, b| b.published.cmp(&a.published));
    allowed.into_iter().next()
}

#[cfg(test)]
#[path = "filter.test.rs"]
mod tests;
