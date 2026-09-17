//! D'où vient un fichier.

use serde::{Deserialize, Serialize};

/// D'où vient un fichier. Détermine l'API à interroger pour ses dépendances :
/// un identifiant de projet Modrinth n'a aucun sens chez CurseForge.
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
