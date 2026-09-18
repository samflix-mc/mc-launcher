//! A tiny HTTP server, for development and nothing else.
//!
//! ## Why not a crate
//!
//! Two hundred lines of `tokio` against one more dependency in a tree that
//! gets audited on every push. This server serves ONLY the developer's
//! machine: it listens on the loopback, it talks to a single client, and it
//! has to withstand nothing — neither load nor an attacker, since it doesn't
//! exist in the distributed binary.
//!
//! What it knows how to do is therefore exactly what's needed: read a
//! request, render JSON, and hold an event stream open. What it doesn't know
//! how to do — HTTPS, `keep-alive`, transfer encodings — it will never meet.
//!
//! ## The split
//!
//! [`Request`] is what the server understood; [`Response`] is what it
//! renders. Both are ordinary structs, which is what makes the routing —
//! which is where the decisions live — testable without opening a socket.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// What a request tells us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
}

/// What we render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub code: u16,
    pub mime_type: String,
    pub body: String,
}

impl Response {
    pub fn json(body: String) -> Response {
        Response {
            code: 200,
            mime_type: "application/json".to_string(),
            body,
        }
    }

    /// An error, in the SAME shape as the one from the Tauri bridge.
    ///
    /// `invoke` rejects with the error value as-is — a string — and the front
    /// handles it via `errorMessage`. Rendering an object of another shape
    /// here would force the front to distinguish between two transports,
    /// which is exactly what this whole module exists to avoid.
    pub fn error(code: u16, message: &str) -> Response {
        Response {
            code,
            mime_type: "application/json".to_string(),
            body: serde_json::to_string(message).unwrap_or_else(|_| "\"error\"".to_string()),
        }
    }

    pub fn empty() -> Response {
        Response::json("null".to_string())
    }
}

/// The request line and headers, parsed.
///
/// Returns `None` on what it can't read: a malformed request shouldn't take
/// the server down.
pub fn parse(raw: &str) -> Option<(String, String, HashMap<String, String>)> {
    let (header, _) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
    let mut lines = header.lines();

    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    // The request may carry a query string; routing doesn't want it.
    let path = target.split('?').next().unwrap_or(&target).to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    Some((method, path, headers))
}

/// How many bytes of body the request announces.
///
/// Zero when the header is missing or doesn't parse: a body we don't expect
/// is better than a wait that never ends.
pub fn announced_length(headers: &HashMap<String, String>) -> usize {
    headers
        .get("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// The text of a response, as it goes out on the socket.
///
/// The cross-origin headers are there because the front runs on Angular's
/// development server, so on ANOTHER port. Without them, the browser refuses
/// the response without anything showing up server-side.
pub fn render(response: &Response) -> String {
    format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        response.code,
        reason(response.code),
        response.mime_type,
        response.body.len(),
        response.body
    )
}

fn reason(code: u16) -> &'static str {
    match code {
        204 => "No Content",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

/// What the server does with a request.
pub type Router =
    Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;

/// Listens, and serves until it's stopped.
///
/// `events` feeds the SSE stream: everything published there goes out to the
/// clients subscribed to `/events`.
pub async fn serve(
    port: u16,
    router: Router,
    events: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(port, "development server listening");

    loop {
        let (stream, _) = listener.accept().await?;
        let router = Arc::clone(&router);
        let events = events.clone();
        tokio::spawn(async move {
            if let Err(error) = handle(stream, router, events).await {
                tracing::debug!(error = %error, "connection closed");
            }
        });
    }
}

async fn handle(
    mut stream: TcpStream,
    router: Router,
    events: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    // We read until the end of the headers, then exactly what
    // `Content-Length` announces. No `keep-alive`: one round trip per
    // connection, which is enough for a single client and removes a whole
    // slice of state machine.
    let (method, path, headers) = loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&buffer).into_owned();
        if text.contains("\r\n\r\n")
            && let Some(parsed) = parse(&text)
        {
            break parsed;
        }
    };

    // The preflight: the front runs on another port, and the browser asks
    // for permission before sending JSON.
    if method == "OPTIONS" {
        let response = Response {
            code: 204,
            mime_type: "text/plain".to_string(),
            body: String::new(),
        };
        stream.write_all(render(&response).as_bytes()).await?;
        return Ok(());
    }

    if path == "/events" {
        return broadcast_events(stream, events).await;
    }

    let expected = announced_length(&headers);
    let text = String::from_utf8_lossy(&buffer).into_owned();
    let already = text.split_once("\r\n\r\n").map(|(_, c)| c).unwrap_or("");
    let mut body = already.as_bytes().to_vec();
    while body.len() < expected {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    let response = router(Request {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
    .await;

    stream.write_all(render(&response).as_bytes()).await
}

/// The event stream, over SSE.
///
/// Replaces Tauri's `listen()`: the front subscribes to it via `EventSource`
/// and receives the same payloads under the same names.
async fn broadcast_events(
    mut stream: TcpStream,
    events: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let header = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: keep-alive\r\n\
         \r\n";
    stream.write_all(header.as_bytes()).await?;

    let mut subscription = events.subscribe();
    loop {
        match subscription.recv().await {
            Ok(payload) => {
                stream
                    .write_all(format!("data: {payload}\n\n").as_bytes())
                    .await?;
                stream.flush().await?;
            }
            // A slow client let messages go by: we keep going rather than
            // disconnecting it. A lost progress update has no consequence —
            // the next one carries the full state, not a delta.
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "http.test.rs"]
mod tests;
