//! De quel côté un mod doit être installé.

/// Côté sur lequel un mod ou une dépendance a un sens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Client,
    Server,
    Both,
}

impl Side {
    /// Côté couvrant les deux usages : un mod tiré côté client *et* côté
    /// serveur doit finir dans les deux dossiers.
    pub fn union(self, other: Side) -> Side {
        if self == other { self } else { Side::Both }
    }

    pub fn includes(self, other: Side) -> bool {
        self == Side::Both || self == other
    }

    pub fn parse(text: &str) -> Option<Side> {
        match text.trim().to_ascii_uppercase().as_str() {
            "CLIENT" => Some(Side::Client),
            "SERVER" | "DEDICATED_SERVER" => Some(Side::Server),
            "BOTH" => Some(Side::Both),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Side::Client => "client",
            Side::Server => "server",
            Side::Both => "both",
        }
    }
}

#[cfg(test)]
#[path = "cote.test.rs"]
mod tests;
