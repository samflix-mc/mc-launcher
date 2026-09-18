//! The build to keep for a request, or the explanation for its absence.

use anyhow::{Result, bail};

use super::Registry;
use super::registry::filter::pick;
use super::request::Request;
use crate::Candidate;

/// described, none of which look at the state of the resolution.
pub(super) async fn choose_build(
    registry: &Registry,
    request: &Request,
    mc: &str,
    loader: &str,
) -> Result<Candidate> {
    if let Some(pinned) = registry.pinned(request, mc, loader).await? {
        return Ok(pinned);
    }

    let found = registry
        .candidates(&request.slug, request.source, mc, loader)
        .await?;
    decide(found, request, mc, loader)
}

/// What we keep from a list of candidates, or why we keep nothing.
///
/// Kept apart from [`choose_build`] for a simple reason: the judgment only
/// depends on what the source answered, never on the network. Mixed with the
/// call, it was untestable — and yet these are the three messages the player
/// will read the day their pack fails to install.
pub(super) fn decide(
    found: Vec<Candidate>,
    request: &Request,
    mc: &str,
    loader: &str,
) -> Result<Candidate> {
    let had_candidates = !found.is_empty();

    let candidate = match pick(found, request) {
        Some(c) => c,
        // The project exists, but nothing matches: say what, or the message
        // sends the reader looking for an absent mod when it's right there.
        None if had_candidates => bail!(
            "{} : no version matches{}",
            request.slug,
            match (&request.version, request.channel) {
                (Some(v), _) => format!(" the requested version \"{v}\""),
                (None, Some(ch)) => format!(" the {} channel or more stable", ch.as_str()),
                _ => " the release channel".to_string(),
            }
        ),
        // Both sources answered and neither knows this project. Without a
        // key, CurseForge's keyword search is closed: a slug that doesn't
        // match the one on the site can't be found there, and that's the
        // most frequent cause of this message.
        None => bail!(
            "{} : not found for Minecraft {mc} / {loader}. \
             Check the slug as it appears in the mod's page address.",
            request.slug
        ),
    };

    if !candidate.redistributable || candidate.url.is_empty() {
        bail!(
            "{} : the author disabled downloads through a third-party launcher. \
             Get the file from {} and drop it in the manual imports folder.",
            candidate.slug,
            candidate.page_url.as_deref().unwrap_or("the mod's page")
        );
    }

    Ok(candidate)
}

#[cfg(test)]
#[path = "choice.test.rs"]
mod tests;

#[cfg(test)]
#[path = "choice.refusal.test.rs"]
mod tests_refusal;
