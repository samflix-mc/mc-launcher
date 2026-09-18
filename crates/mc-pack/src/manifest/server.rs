//! A server's address, as a manifest declares it.

use serde::{Deserialize, Serialize};

/// A server's address, as `--quickPlayMultiplayer` expects it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    /// Absent = 25565, Minecraft's default port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

impl Server {
    /// The address as `--quickPlayMultiplayer` reads it.
    ///
    /// Minecraft hands this string to Guava's `HostAndPort`, which rejects
    /// anything carrying more than one “:” without brackets. An IPv6
    /// address carries at least two: written bare, it doesn't give a wrong
    /// address, it gives none at all, and the game opens on the menu
    /// without announcing anything. Hence the brackets, applied here rather
    /// than expected from whoever writes the manifest.
    ///
    /// The port is written as soon as it's declared, even when it's 25565.
    /// Omitting it looks harmless, but Minecraft doesn't just compose the
    /// address: without a port, it queries the “_minecraft._tcp.<host>” SRV
    /// record before falling back to the default. Knowing whether the two
    /// forms lead to the same server requires knowing the DNS zone, which
    /// the launcher doesn't see. So we don't discard what the manifest took
    /// the trouble to say.
    pub fn address(&self) -> String {
        let host = self.host.trim();
        let host = if is_bare_ipv6(host) {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        match self.port {
            Some(port) => format!("{host}:{port}"),
            None => host,
        }
    }
}

/// An IPv6 address written without its brackets.
///
/// At least two “:”: an IPv6 address always contains at least two, where
/// “host:port” has only one. That's the distinction Guava makes, so it's
/// the one Minecraft makes.
fn is_bare_ipv6(host: &str) -> bool {
    !host.starts_with('[') && host.matches(':').count() >= 2
}

/// Does the host already carry a “:port”?
///
/// Adding a second one would give “host:25565:25566”, which Guava rejects
/// just as it rejects a bare IPv6 address.
pub(super) fn already_has_a_port(host: &str) -> bool {
    match host.rsplit_once(':') {
        Some((before, after)) => {
            (before.ends_with(']') || !before.contains(':')) && after.parse::<u16>().is_ok()
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "server.test.rs"]
mod tests;
