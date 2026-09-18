//! A local HTTP server, for exercising whatever talks to the network.
//!
//! Everything the launcher downloads comes from five APIs — Mojang, Modrinth,
//! CurseForge, NeoForge, Adoptium — and half the repo's uncovered code is
//! made of calls to them. Testing them meant either going out on the network
//! on every `cargo test`, or making the responses fabricable.
//!
//! Going out on the network was out of the question: CI would hit a
//! CurseForge outage, the tests would burn through quotas, and above all
//! nothing would let us trigger the cases that matter — a 500 that succeeds
//! on the second try, a rejected key, a truncated file, a JSON that no one
//! publishes anymore.
//!
//! Hence this server: it listens on an ephemeral local address, returns
//! whatever it's been told to return, and records what it was asked. It
//! speaks just enough HTTP/1.1 to satisfy `reqwest`.
//!
//! ```no_run
//! # async fn example() {
//! let server = mc_testkit::Server::new().await;
//! server.json("/v2/project/jei", r#"{"slug":"jei"}"#);
//! let url = server.url("/v2/project/jei");
//! # }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// What the server will return for a path.
#[derive(Clone)]
struct Response {
    code: u16,
    body: Vec<u8>,
    content_type: String,
    /// Number of error responses to serve before the real one.
    ///
    /// It's the only way to exercise retries: a transient CDN outage can't
    /// be commanded on demand, and without this the retry loop would never
    /// run more than once.
    failures_remaining: u32,
}

/// A received request, in the shape we want to be able to assert against
/// afterward.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    /// Value of a header, case-insensitive — CurseForge's key goes through
    /// `x-api-key`, and nothing guarantees the case on the way back.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[derive(Default)]
struct State {
    routes: HashMap<String, Response>,
    received: Vec<Request>,
}

pub struct Server {
    address: SocketAddr,
    state: Arc<Mutex<State>>,
}

impl Server {
    /// Opens a server on a free port of the loopback interface.
    pub async fn new() -> Server {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("no free port on the loopback interface");
        let address = listener.local_addr().unwrap();
        let state = Arc::new(Mutex::new(State::default()));

        let shared = Arc::clone(&state);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let state = Arc::clone(&shared);
                tokio::spawn(async move {
                    serve(stream, state).await;
                });
            }
        });

        Server { address, state }
    }

    /// The server's root, to substitute for the target API's URL.
    pub fn base(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base())
    }

    /// Responds with JSON on this path.
    pub fn json(&self, path: &str, body: &str) -> &Self {
        self.set(path, 200, body.as_bytes(), "application/json")
    }

    /// Responds with raw bytes — a jar, an asset file.
    pub fn bytes(&self, path: &str, body: &[u8]) -> &Self {
        self.set(path, 200, body, "application/octet-stream")
    }

    /// Responds with an error code, no useful body.
    pub fn code(&self, path: &str, code: u16) -> &Self {
        self.set(path, code, b"error", "text/plain")
    }

    /// Responds with an error code and a chosen body — some APIs say in the
    /// body what their code doesn't.
    pub fn code_with(&self, path: &str, code: u16, body: &str) -> &Self {
        self.set(path, code, body.as_bytes(), "application/json")
    }

    /// Fails `how_many` times, then responds normally.
    pub fn fails_then(&self, path: &str, how_many: u32, body: &str) -> &Self {
        self.set(path, 200, body.as_bytes(), "application/json");
        if let Some(response) = self.state.lock().unwrap().routes.get_mut(path) {
            response.failures_remaining = how_many;
        }
        self
    }

    fn set(&self, path: &str, code: u16, body: &[u8], content_type: &str) -> &Self {
        self.state.lock().unwrap().routes.insert(
            path.to_string(),
            Response {
                code,
                body: body.to_vec(),
                content_type: content_type.to_string(),
                failures_remaining: 0,
            },
        );
        self
    }

    /// Everything that was requested, in order.
    pub fn received(&self) -> Vec<Request> {
        self.state.lock().unwrap().received.clone()
    }

    /// Number of requests received on a path, query string included.
    pub fn calls(&self, path: &str) -> usize {
        self.received()
            .iter()
            .filter(|r| r.path == path || r.path.starts_with(&format!("{path}?")))
            .count()
    }
}

async fn serve(mut stream: tokio::net::TcpStream, state: Arc<Mutex<State>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut raw = Vec::new();
    let mut buffer = [0u8; 4096];
    // The header ends with a blank line; the body, if there is one, follows
    // for the announced length.
    let boundary = loop {
        let read_len = match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        raw.extend_from_slice(&buffer[..read_len]);
        if let Some(position) = find(&raw, b"\r\n\r\n") {
            break position;
        }
    };

    let header_text = String::from_utf8_lossy(&raw[..boundary]).to_string();
    let mut lines = header_text.lines();
    let first_line = lines.next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let expected: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = raw[boundary + 4..].to_vec();
    while body.len() < expected {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(n) => body.extend_from_slice(&buffer[..n]),
        }
    }

    // The path serves as the key without its query string: callers put
    // parameters in it that we don't want to have to reproduce exactly.
    let key = path.split('?').next().unwrap_or(&path).to_string();
    let (code, payload, content_type) = {
        let mut state = state.lock().unwrap();
        state.received.push(Request {
            method,
            path: path.clone(),
            headers,
            body: String::from_utf8_lossy(&body).to_string(),
        });
        match state.routes.get_mut(&key) {
            None => (404, b"nothing here".to_vec(), "text/plain".to_string()),
            Some(response) if response.failures_remaining > 0 => {
                response.failures_remaining -= 1;
                (500, b"temporary outage".to_vec(), "text/plain".to_string())
            }
            Some(response) => (
                response.code,
                response.body.clone(),
                response.content_type.clone(),
            ),
        }
    };

    let head = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reason(code),
        payload.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.write_all(&payload).await;
    let _ = stream.flush().await;
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|f| f == needle)
}

fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Status",
    }
}
