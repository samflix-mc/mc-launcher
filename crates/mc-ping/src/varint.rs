//! Minecraft's VarInt: 7 payload bits per byte, the 8th says "more follows".
//!
//! A 32-bit value never needs more than 5 bytes this way. That bound is
//! also the safety net: nothing in the protocol produces a longer one, so a
//! 6th continuation byte only ever comes from a server that isn't playing
//! by the rules, and [`decode`] refuses it outright rather than reading on.

use anyhow::{Result, bail};

/// No 32-bit VarInt is longer than this.
const MAX_BYTES: usize = 5;

/// Encodes a VarInt, least significant group first.
///
/// Negative values encode from their `u32` bit pattern, exactly as the
/// protocol expects: `-1` — the "unknown protocol version" this crate
/// sends — comes out as all ones, the widest VarInt there is.
pub(crate) fn encode(value: i32) -> Vec<u8> {
    let mut remaining = value as u32;
    let mut out = Vec::with_capacity(MAX_BYTES);
    loop {
        let mut byte = (remaining & 0x7F) as u8;
        remaining >>= 7;
        if remaining != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if remaining == 0 {
            break;
        }
    }
    out
}

/// Decodes a VarInt from the start of `buf`.
///
/// Three outcomes, and callers reading off a socket need all three:
/// - `Ok(Some((value, len)))`: complete, `len` bytes consumed.
/// - `Ok(None)`: `buf` ends before a terminating byte, but in fewer than
///   [`MAX_BYTES`] — read more and try again.
/// - `Err`: [`MAX_BYTES`] bytes seen and still no terminator. A real VarInt
///   never needs a 6th; this is what stops us from looping on one that
///   claims to.
pub(crate) fn decode(buf: &[u8]) -> Result<Option<(i32, usize)>> {
    let mut value: u32 = 0;
    for (index, &byte) in buf.iter().enumerate() {
        if index >= MAX_BYTES {
            bail!("VarInt longer than {MAX_BYTES} bytes");
        }
        value |= u32::from(byte & 0x7F) << (7 * index);
        if byte & 0x80 == 0 {
            return Ok(Some((value as i32, index + 1)));
        }
    }
    if buf.len() >= MAX_BYTES {
        bail!("VarInt longer than {MAX_BYTES} bytes");
    }
    Ok(None)
}

#[cfg(test)]
#[path = "varint.test.rs"]
mod tests;
