//! Which side a mod must be installed on.

/// Side on which a mod or a dependency makes sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Client,
    Server,
    Both,
}

impl Side {
    /// Side covering both uses: a mod pulled in on the client *and* the
    /// server must end up in both folders.
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
#[path = "side.test.rs"]
mod tests;
