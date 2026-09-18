//! Whether the server this pack declares answers, and to how many players.
//!
//! ## No SRV resolution
//!
//! Minecraft queries `_minecraft._tcp.<host>` when the manifest declares no
//! port, and only falls back to `25565` if that lookup fails. This probe
//! does neither: absent a declared port, it goes straight to `25565`. A
//! server that relies on an SRV record to route elsewhere will therefore
//! show up here as offline even though the game reaches it fine —
//! implementing SRV resolution is a separate piece of work, not started by
//! this module.

use std::time::Duration;

use serde::Serialize;

use mc_pack::manifest::Server;

/// Minecraft's default port, used whenever the manifest doesn't declare
/// one — see the "no SRV resolution" note above for what that costs.
const DEFAULT_PORT: u16 = 25565;

/// How long the panel is willing to wait. Short on purpose: this runs on
/// Spawn opening and every thirty seconds after, and must never be what
/// makes the screen feel frozen.
const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerState {
    Online,
    Offline,
    /// The pack installed on this machine has no server for this binary's
    /// environment — preproduction, for instance, never has one.
    Undeclared,
}

/// The server panel's whole contract with the front end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub state: ServerState,
    /// Empty when [`ServerState::Undeclared`]: there's nothing to show.
    pub host: String,
    pub players: Option<u32>,
    pub slots: Option<u32>,
    pub version: Option<String>,
}

impl ServerStatus {
    fn undeclared() -> Self {
        Self {
            state: ServerState::Undeclared,
            host: String::new(),
            players: None,
            slots: None,
            version: None,
        }
    }

    fn offline(host: String) -> Self {
        Self {
            state: ServerState::Offline,
            host,
            players: None,
            slots: None,
            version: None,
        }
    }
}

/// Probes the server this pack declares, and never fails.
///
/// Neither a missing declaration nor an unreachable server is an error: the
/// first is normal outside production, and the second is what a server
/// being down looks like. Turning either into a `Result::Err` would send it
/// to Sentry through the same path a real bridge failure takes, and a
/// player's router blocking outbound 25565 is not an incident.
#[tauri::command]
pub async fn server_status() -> ServerStatus {
    let Some(server) = declared_server() else {
        return ServerStatus::undeclared();
    };

    let host = server.address();
    let port = server.port.unwrap_or(DEFAULT_PORT);

    from_ping(host, mc_ping::ping(&server.host, port, TIMEOUT).await)
}

/// Turns a probe's outcome into what the panel shows.
///
/// Split out from [`server_status`] because it's the only part of this
/// module with a decision in it — reading the manifest and dialing the
/// socket don't need a test to know what they do, telling `Ok` from `Err`
/// apart does.
fn from_ping(host: String, outcome: anyhow::Result<mc_ping::Status>) -> ServerStatus {
    match outcome {
        Ok(status) => ServerStatus {
            state: ServerState::Online,
            host,
            players: status.online,
            slots: status.max,
            version: status.version,
        },
        Err(error) => {
            // `debug`, not `warn`: a server being off is the ordinary case
            // between two publications, not a degraded one, and this runs
            // every thirty seconds for as long as Spawn stays open.
            tracing::debug!(host, error = %error, "server not reachable, shown offline");
            ServerStatus::offline(host)
        }
    }
}

/// The server this binary's environment declares, read off the pack
/// installed on THIS machine.
///
/// The LOCAL pack, same as `game::target` — never the published one:
/// probing an address the disk doesn't have would tell the player a server
/// is reachable that the game itself won't join. `None` covers both "no
/// pack installed yet" and "installed, but this environment has no server",
/// deliberately: the panel treats them the same way.
fn declared_server() -> Option<Server> {
    let (source, _options) = crate::cinematic::where_to_install();
    let pack = source.load_local().ok()?;
    pack.manifest
        .server_for(mc_log::environment::current())
        .cloned()
}

#[cfg(test)]
#[path = "server.test.rs"]
mod tests;
