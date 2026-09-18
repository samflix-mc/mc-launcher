//! What a status response says, and what we keep from it.
//!
//! `description` — the MOTD, a string or a tree of chat components — isn't
//! modeled here at all. Rendering it properly is a separate piece of work,
//! and this crate's one caller doesn't need it: not naming the field is
//! enough for `serde` to ignore it, whichever shape it comes in.

use anyhow::{Context, Result};
use serde::Deserialize;

/// What a Server List Ping answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// How many players are connected. `None` when the server didn't say —
    /// which is NOT zero, and the two must not be confused: a panel showing
    /// "0 / 0" for a server that answered without a `players` object claims
    /// to know something it doesn't.
    pub online: Option<u32>,
    /// How many slots. `None` on the same terms as [`Status::online`].
    pub max: Option<u32>,
    pub version: Option<String>,
    /// Time from opening the connection to reading this response, in
    /// milliseconds.
    pub latency_ms: u64,
}

#[derive(Debug, Deserialize)]
struct Raw {
    players: Option<RawPlayers>,
    version: Option<RawVersion>,
}

#[derive(Debug, Deserialize)]
struct RawPlayers {
    online: Option<u32>,
    max: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawVersion {
    name: Option<String>,
}

/// Reads the three fields the launcher shows, failing on nothing but JSON
/// that doesn't parse.
///
/// `players`, `version`, and each of their sub-fields are all optional in
/// the protocol — a server that leaves one out hasn't sent an invalid
/// response, and the probe must not treat it as a failure. Only unparsable
/// JSON is: at that point there's nothing left to be lenient about.
pub(crate) fn parse(json: &str, latency_ms: u64) -> Result<Status> {
    let raw: Raw = serde_json::from_str(json).context("status JSON")?;
    Ok(Status {
        online: raw.players.as_ref().and_then(|players| players.online),
        max: raw.players.as_ref().and_then(|players| players.max),
        version: raw.version.and_then(|version| version.name),
        latency_ms,
    })
}

#[cfg(test)]
#[path = "status.test.rs"]
mod tests;
