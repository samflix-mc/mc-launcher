//! Look up a project's numeric id from its slug.

use anyhow::{Context, Result};

use super::CurseForgeWeb;
use super::api::Widget;

impl CurseForgeWeb {
    pub(super) async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        // 403 is Cloudflare's response just as much as a closed route: in
        // both cases this source has nothing to offer, and the caller must
        // be able to carry on without everything stopping.
        if !response.status().is_success() {
            tracing::debug!(url, status = %response.status(), "site route gave no response");
            return Ok(None);
        }

        // A response that arrives but doesn't parse is **not** the same
        // thing as an absence, and confusing the two is costly: a forgotten
        // `data` envelope once made a genuinely existing pinned build get
        // declared missing, without a single line saying so. `None` is still
        // returned — the site sometimes serves a Cloudflare page as HTTP
        // 200, and stopping everything would make the fallback unusable —
        // but it's logged.
        match response.json().await {
            Ok(parsed) => Ok(Some(parsed)),
            Err(error) => {
                tracing::warn!(url, error = %error, "unreadable response from the site");
                Ok(None)
            }
        }
    }

    /// Project id from a slug.
    ///
    /// The site's own search returns 403; cfwidget is the only path left. It
    /// returns 202 while it builds its cache for a project it has never
    /// seen, hence the second attempt.
    pub(super) async fn project_id(&self, slug: &str) -> Result<Option<(u32, String)>> {
        for attempt in 0..2 {
            tokio::time::sleep(cache_delay(attempt)).await;
            let response = self
                .dl
                .client()
                .get(format!("{}/{slug}", self.widget))
                .send()
                .await
                .with_context(|| format!("cfwidget for {slug}"))?;

            if response.status() == reqwest::StatusCode::ACCEPTED {
                continue;
            }
            if !response.status().is_success() {
                return Ok(None);
            }
            let widget: Widget = match response.json().await {
                Ok(w) => w,
                Err(_) => return Ok(None),
            };
            return Ok(Some((widget.id, widget.title)));
        }
        Ok(None)
    }

    /// Resolves a slug or a numeric id into `(id, name)`.
    pub(super) async fn resolve_project(&self, id_or_slug: &str) -> Result<Option<(u32, String)>> {
        match id_or_slug.parse::<u32>() {
            // A numeric id comes from an already-resolved dependency: the
            // readable name isn't essential, and skipping it saves a call to
            // a third-party service.
            Ok(id) => Ok(Some((id, format!("project {id}")))),
            Err(_) => self.project_id(id_or_slug).await,
        }
    }
}

/// Time left for cfwidget to build its cache before attempt `attempt`, the
/// first one being zero.
///
/// It returns 202 for a project it has never seen, and builds it in the
/// background. Three seconds are enough in practice; the first attempt
/// doesn't wait, which is expressed here as a product by zero rather than a
/// condition — a zero duration can be checked, a skipped branch can't be
/// seen.
fn cache_delay(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_secs(3) * attempt
}

#[cfg(test)]
#[path = "project.test.rs"]
mod tests;
