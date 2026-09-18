use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::exchange;
use crate::varint;

/// A valid response packet: length prefix, id `0`, then the JSON string.
fn response_packet(json: &str) -> Vec<u8> {
    let mut payload = varint::encode(0);
    payload.extend_from_slice(&varint::encode(json.len() as i32));
    payload.extend_from_slice(json.as_bytes());

    let mut framed = varint::encode(payload.len() as i32);
    framed.extend_from_slice(&payload);
    framed
}

async fn listener() -> (TcpListener, u16) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

#[tokio::test]
async fn a_reachable_server_answers_with_its_status() {
    let (listener, port) = listener().await;

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        // The handshake and status request: read and discard, this fake
        // server answers unconditionally, which is enough to exercise the
        // client side of the exchange.
        let mut discard = [0u8; 256];
        let _ = stream.read(&mut discard).await;

        let response =
            response_packet(r#"{"players":{"online":4,"max":20},"version":{"name":"1.21.1"}}"#);
        stream.write_all(&response).await.unwrap();
    });

    let status = exchange("127.0.0.1", port).await.expect("a valid response");

    assert_eq!(status.online, Some(4));
    assert_eq!(status.max, Some(20));
    assert_eq!(status.version.as_deref(), Some("1.21.1"));
}

/// The timeout is [`crate::ping`]'s job, not [`exchange`]'s — this is
/// exercised end to end in `lib.test.rs`, where a silent server proves the
/// deadline bounds the WHOLE exchange rather than each read.
#[tokio::test]
async fn a_server_that_closes_immediately_fails_without_hanging() {
    let (listener, port) = listener().await;

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    });

    let started = std::time::Instant::now();
    let result = exchange("127.0.0.1", port).await;

    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "a closed connection should fail immediately, not hang"
    );
}

#[tokio::test]
async fn an_oversized_announced_length_is_refused_before_reading_it() {
    let (listener, port) = listener().await;

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut discard = [0u8; 256];
        let _ = stream.read(&mut discard).await;

        // Announces far more than the crate allows, and never sends the
        // body: a well-behaved probe refuses on the announcement alone.
        let oversized = varint::encode(500_000);
        let _ = stream.write_all(&oversized).await;
        // Held open so a probe that ignored the announcement would hang
        // instead of erroring — the test would then time out, not pass.
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let started = std::time::Instant::now();
    let result = exchange("127.0.0.1", port).await;

    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the size bound must reject before the far-off sleep ends"
    );
}
