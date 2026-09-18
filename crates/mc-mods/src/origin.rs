//! Where a file comes from.

use serde::{Deserialize, Serialize};

/// Where a file comes from. Determines which API to query for its
/// dependencies: a Modrinth project ID means nothing to CurseForge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Modrinth,
    CurseForge,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Modrinth => "modrinth",
            Origin::CurseForge => "curseforge",
        }
    }
}
