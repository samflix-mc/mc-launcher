//! Minecraft's Server List Ping — the same probe a client sends to fill in
//! a server's line in the multiplayer menu, told apart from a real login
//! only by the handshake's `next_state`.
//!
//! ## The exchange
//!
//! One TCP connection, three packets, in order:
//!
//! 1. **Handshake**, packet id `0x00`: protocol version, host, port, next
//!    state. The protocol version is sent as `-1` — "unknown" — since this
//!    probe doesn't claim to speak a particular one; a vanilla server
//!    answers the status query regardless.
//! 2. **Status request**, packet id `0x00`, carrying nothing else.
//! 3. The server's answer: one packet, id `0x00`, a single JSON string.
//!
//! Every packet is a VarInt length, then a payload starting with a VarInt
//! id; a string inside a payload is a VarInt byte length, then UTF-8. See
//! [`varint`] for the encoding and [`protocol`] for how the three packets
//! above are built and read, as PURE functions — the reason the protocol is
//! testable without ever opening a socket. [`connection`] is the one module
//! that does.
//!
//! ## What's deliberately absent: SRV resolution
//!
//! A Minecraft client queries `_minecraft._tcp.<host>` when it has no port
//! to go on, and only falls back to `25565` if that lookup fails. This
//! crate does neither: [`ping`] dials the `host` and `port` it's given,
//! nothing more. A caller with no declared port has to decide what to probe
//! itself — see `mc-app`'s `server_status` command, which is where that
//! decision, and its consequence, are written down.

mod connection;
mod protocol;
mod status;
mod varint;

use std::time::Duration;

use anyhow::{Result, anyhow};

pub use status::Status;

/// Probes a server, end to end, within `timeout`.
///
/// The deadline wraps the WHOLE exchange — connecting included — not each
/// individual read. A server that accepts the connection and then never
/// answers must not be allowed to hold this call open past `timeout` just
/// because no single read on its own exceeded it.
///
/// `timeout` is a parameter and not a constant on purpose: a status panel
/// and a "is this address even right" diagnostic don't share a good
/// default, and this crate doesn't get to pick for both.
pub async fn ping(host: &str, port: u16, timeout: Duration) -> Result<Status> {
    tokio::time::timeout(timeout, connection::exchange(host, port))
        .await
        .map_err(|_| anyhow!("{host}:{port} did not answer within {timeout:?}"))?
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
