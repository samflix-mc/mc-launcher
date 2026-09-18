//! The HTTP client, its retries, and what it writes to disk.

pub(crate) mod file;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use std::time::Duration;

use crate::progress::{Observer, Progress};

/// HTTP client shared by the whole launcher.
pub struct Downloader {
    client: reqwest::Client,
    retries: u32,
    observer: Option<Observer>,
}

impl Downloader {
    /// Both public APIs in use require an identifiable `User-Agent`:
    /// Modrinth documents `project/version (contact)` and applies a rate
    /// limit per agent, CurseForge logs the agent along with the key.
    pub fn new(user_agent: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            client,
            retries: 3,
            observer: None,
        })
    }

    /// Attaches a progress observer.
    ///
    /// Separate from [`Downloader::new`] so that the command line, which
    /// doesn't need one, has nothing to pass. Without an observer, everything
    /// below reduces to a check on an `Option` per chunk received.
    #[must_use]
    pub fn observe(mut self, observer: Observer) -> Self {
        self.observer = Some(observer);
        self
    }

    /// Makes known where the download stands.
    ///
    /// Public because batches aren't counted here: only the caller, who has
    /// the list of assets or mods, knows how many files and bytes it's about
    /// to request, and it's this announcement that makes a time remaining
    /// computable.
    pub fn emit(&self, progress: Progress<'_>) {
        if let Some(observer) = &self.observer {
            observer(progress);
        }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Response body, with retries on network error or 5xx.
    pub async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut last = None;
        for attempt in 0..self.retries {
            tokio::time::sleep(backoff(attempt)).await;
            match self.try_bytes(url).await {
                Ok(b) => {
                    tracing::trace!(url, bytes = b.len(), attempt = attempt + 1, "GET");
                    return Ok(b);
                }
                // An absent one is NOT retried. Asking three times for an
                // address that answered "it's not there" costs two
                // round-trips and two backoff waits for the same answer —
                // and on the news feed, which loads when a page opens, it
                // shows.
                Err(e) if e.is::<Absent>() => return Err(e),
                Err(e) => {
                    // A retry that eventually succeeds surfaces nowhere
                    // else: it's nonetheless the first sign of a
                    // degrading source.
                    tracing::warn!(url, attempt = attempt + 1, error = %e, "failed, retrying");
                    last = Some(e);
                }
            }
        }
        Err(last.expect("at least one attempt")).with_context(|| format!("GET {url}"))
    }

    /// A single attempt, read chunk by chunk.
    ///
    /// `response.bytes()` would return the whole body at once, which is
    /// shorter to write and shows nothing: no intermediate byte is
    /// observable, hence no rate and no time remaining. Streaming costs a
    /// loop and makes the wait legible.
    ///
    /// What a failed attempt had counted is removed before returning control
    /// — the retry will count it again.
    async fn try_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if status == 404 {
            // A RESPONSE, not a failure — see [`Absent`].
            return Err(Absent {
                url: url.to_string(),
            }
            .into());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!(
                "HTTP {status}: {}",
                body.chars().take(300).collect::<String>()
            );
        }

        // The announced size avoids reallocating the buffer a dozen times
        // over a fifty-megabyte jar.
        let mut received = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    self.emit(Progress::Lost(received.len() as u64));
                    return Err(error.into());
                }
            };
            self.emit(Progress::Received(chunk.len() as u64));
            received.extend_from_slice(&chunk);
        }
        Ok(received)
    }
}

/// Wait before attempt number `attempt`, the first one being zero.
///
/// Short and growing backoff: a CDN's 5xx errors pass within a few seconds,
/// and a modpack makes thousands of requests — waiting a second on each one
/// would cost more than the errors it avoids. The first attempt doesn't
/// wait, which is expressed here as a product by zero rather than a
/// condition: a zero duration can be checked, a skipped branch can't be
/// seen.
fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(400 * u64::from(attempt))
}

/// The resource doesn't exist: **a response, not a failure**.
///
/// The distinction isn't a subtlety. A host that returns 404 on
/// `news.json` has answered: it doesn't publish a feed. An unreachable
/// host, on the other hand, said nothing — and it's at that point, and only
/// at that point, that a local copy must take over and that a message makes
/// sense.
///
/// Confusing the two made a news page show an error on every host that
/// doesn't publish one yet — that is, on all three, the day the page was
/// written.
#[derive(Debug)]
pub struct Absent {
    pub url: String,
}

impl std::fmt::Display for Absent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The code is in there: a log is read out of habit, and "404" is
        // what you look for.
        write!(f, "HTTP 404: {} does not exist on this host", self.url)
    }
}

impl std::error::Error for Absent {}

#[cfg(test)]
#[path = "download.test.rs"]
mod tests;
