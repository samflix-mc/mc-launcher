//! The one module that opens a socket.
//!
//! Everything that decides what bytes mean lives in [`crate::protocol`] and
//! [`crate::status`], as pure functions. What's left here is genuinely
//! I/O-shaped and hard to make pure: writing to a stream, and reading until
//! a full packet has arrived — or refusing to.

use std::time::Instant;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{protocol, status, varint};

/// Above this, a status response is refused rather than read.
///
/// `Downloader::bytes`, elsewhere in this repo, has no such bound and holds
/// the whole body in memory — a known gap, not one to repeat here. A status
/// JSON is a few hundred bytes even with a long MOTD and a player sample;
/// 256 KiB leaves generous room while still refusing a host that just keeps
/// sending.
const MAX_RESPONSE: usize = 256 * 1024;

/// Connects, exchanges the handshake and status request, and reads the
/// answer back. No deadline of its own — [`crate::ping`] wraps the whole
/// thing in one.
pub(crate) async fn exchange(host: &str, port: u16) -> Result<status::Status> {
    let started = Instant::now();

    let mut stream = TcpStream::connect((host, port))
        .await
        .with_context(|| format!("connecting to {host}:{port}"))?;

    stream
        .write_all(&protocol::handshake(-1, host, port, 1))
        .await
        .context("sending the handshake")?;
    stream
        .write_all(&protocol::status_request())
        .await
        .context("sending the status request")?;

    let payload = read_packet(&mut stream).await?;
    let json = protocol::decode_status_payload(&payload)?;

    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    status::parse(&json, latency_ms)
}

/// Reads one length-prefixed packet, refusing anything announcing more than
/// [`MAX_RESPONSE`] before reading a single byte of it.
///
/// Returns the packet's payload — id included; [`protocol::decode_status_payload`]
/// is what strips that.
async fn read_packet(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];

    let (len, header_len) = loop {
        if let Some(found) = varint::decode(&buf)? {
            break found;
        }
        fill(stream, &mut buf, &mut chunk).await?;
    };

    let len = usize::try_from(len).context("negative packet length")?;
    if len > MAX_RESPONSE {
        bail!("status response announces {len} bytes, beyond the {MAX_RESPONSE} allowed");
    }

    while buf.len() < header_len + len {
        fill(stream, &mut buf, &mut chunk).await?;
    }

    Ok(buf[header_len..header_len + len].to_vec())
}

/// Reads whatever's available into `buf`. An empty read means the other
/// side closed the connection — not something to loop on.
async fn fill(stream: &mut TcpStream, buf: &mut Vec<u8>, chunk: &mut [u8]) -> Result<()> {
    let read = stream.read(chunk).await.context("reading the response")?;
    if read == 0 {
        bail!("connection closed before a full response arrived");
    }
    buf.extend_from_slice(&chunk[..read]);
    Ok(())
}

#[cfg(test)]
#[path = "connection.test.rs"]
mod tests;
