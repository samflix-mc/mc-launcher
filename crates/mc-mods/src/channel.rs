//! How far down a release's stability to go.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Release,
    Beta,
    Alpha,
}

impl Channel {
    pub fn parse(text: &str) -> Channel {
        match text.trim().to_ascii_lowercase().as_str() {
            "release" => Channel::Release,
            "beta" => Channel::Beta,
            _ => Channel::Alpha,
        }
    }

    /// Is `self` acceptable when the manifest allows at most `limit`?
    pub fn allowed_by(self, limit: Channel) -> bool {
        self <= limit
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Release => "release",
            Channel::Beta => "beta",
            Channel::Alpha => "alpha",
        }
    }
}

#[cfg(test)]
#[path = "channel.test.rs"]
mod tests;
