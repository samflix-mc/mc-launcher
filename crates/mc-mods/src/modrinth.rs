//! Modrinth source — consulted first, without an API key.
//!
//! Modrinth takes priority for three concrete reasons: its API is open (no
//! key to distribute with the launcher), it publishes both the SHA-1 and the
//! SHA-512 of every file, and it exposes `client_side` / `server_side` per
//! project — which gives the client/server split without having to enter it
//! by hand in the manifest.
//!
//! It does require an identifiable `User-Agent` in return, and enforces a
//! rate limit; batched calls are therefore preferred over loops of requests.

mod api;
mod conversion;
mod requests;
mod search;

use std::sync::Arc;

pub(crate) const API: &str = "https://api.modrinth.com/v2";

/// The Modrinth client: priority source, and the only one that publishes a SHA-512.
pub struct Modrinth {
    pub(crate) dl: Arc<mc_dl::Downloader>,
    /// API root. Fixed once at construction, and substitutable: tests can't
    /// depend on Modrinth's availability, nor trigger a 500 or a truncated
    /// response from it.
    pub(crate) base: String,
}

impl Modrinth {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self::with_base(dl, API)
    }

    pub(crate) fn with_base(dl: Arc<mc_dl::Downloader>, base: &str) -> Self {
        Self {
            dl,
            base: base.to_string(),
        }
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}
