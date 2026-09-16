//! Jusqu'où descendre dans la stabilité d'une publication.

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

    /// `self` est-il acceptable quand le manifeste autorise au plus `limit` ?
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
#[path = "canal.test.rs"]
mod tests;
