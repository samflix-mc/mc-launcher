//! Building the two packets we send, and reading the one we get back.
//!
//! Every function here is PURE — bytes in, bytes or a value out, no socket
//! anywhere in sight. That's what lets the protocol be tested without
//! opening one: [`crate::connection`] is the only module that does I/O, and
//! all it does is fill a buffer and hand it here.

use anyhow::{Context, Result, bail};

use crate::varint;

/// A packet, framed the way the protocol expects: its own length as a
/// VarInt, then the payload — which already carries the packet id.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = varint::encode(payload.len() as i32);
    out.extend_from_slice(payload);
    out
}

/// A protocol string: a VarInt byte length, then UTF-8. Not a character
/// count — a mod's server name in another script would disagree with
/// `str::len` otherwise.
fn write_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&varint::encode(value.len() as i32));
    out.extend_from_slice(value.as_bytes());
}

/// The handshake packet, id `0x00`: protocol version, host, port, then the
/// next state — `1` asks for status, `2` would ask to log in.
pub(crate) fn handshake(protocol_version: i32, host: &str, port: u16, next_state: i32) -> Vec<u8> {
    let mut payload = varint::encode(0);
    payload.extend_from_slice(&varint::encode(protocol_version));
    write_string(&mut payload, host);
    payload.extend_from_slice(&port.to_be_bytes());
    payload.extend_from_slice(&varint::encode(next_state));
    frame(&payload)
}

/// The status request packet, id `0x00`, carrying nothing else.
pub(crate) fn status_request() -> Vec<u8> {
    frame(&varint::encode(0))
}

/// Extracts the status JSON from a response packet's payload.
///
/// `payload` is what's left once the OUTER length prefix has already been
/// stripped by [`crate::connection`] — this function reads the id, then the
/// string that follows it.
pub(crate) fn decode_status_payload(payload: &[u8]) -> Result<String> {
    let (id, id_len) = varint::decode(payload)
        .context("packet id")?
        .ok_or_else(|| anyhow::anyhow!("packet id: truncated"))?;
    if id != 0 {
        bail!("unexpected packet id {id}, expected 0 (status response)");
    }

    let rest = &payload[id_len..];
    let (string_len, len_len) = varint::decode(rest)
        .context("status string length")?
        .ok_or_else(|| anyhow::anyhow!("status string length: truncated"))?;
    let string_len = usize::try_from(string_len).context("negative status string length")?;

    let rest = &rest[len_len..];
    if rest.len() < string_len {
        bail!(
            "status string: {} of {string_len} announced bytes present",
            rest.len()
        );
    }

    String::from_utf8(rest[..string_len].to_vec()).context("status string: not UTF-8")
}

#[cfg(test)]
#[path = "protocol.test.rs"]
mod tests;
