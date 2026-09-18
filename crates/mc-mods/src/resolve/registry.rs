//! The sources, queried in order.

mod curseforge;
pub(super) mod filter;
mod search;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Who listens to RESOLUTION progress.
///
/// Distinct from `mc_dl::Observer`, which counts bytes: here we count queried
/// requests. The two don't overlap — resolution spends most of its time
/// waiting on API responses of a few kilobytes, which doesn't move a byte
/// counter at all while it can last tens of seconds.
///
/// `(done, total)`, where the total is an estimate that can grow.
pub type Progress = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// The two sources, queried in order.
pub struct Registry {
    pub(super) dl: Arc<mc_dl::Downloader>,
    pub(super) modrinth: crate::modrinth::Modrinth,
    pub(super) curseforge_web: crate::curseforge_web::CurseForgeWeb,
    pub(super) cache: PathBuf,
    progress: Option<Progress>,
}

impl Registry {
    pub fn new(cache: PathBuf) -> Result<Self> {
        Self::assemble(cache, None)
    }

    /// The same registry, that reports where its downloads stand.
    ///
    /// The observer is attached at construction, not afterwards: the HTTP
    /// client is shared by both sources behind an `Arc`, and is no longer
    /// mutable once the registry is assembled.
    pub fn observed(cache: PathBuf, observer: mc_dl::Observer) -> Result<Self> {
        Self::assemble(cache, Some(observer))
    }

    /// The same, that also reports where its RESOLUTION stands.
    ///
    /// Attached afterwards, not at assembly: unlike the download observer, it
    /// doesn't have to cross the shared HTTP client.
    pub fn announcing(mut self, progress: Progress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Reports where resolution stands, if anyone is listening.
    pub(super) fn announce(&self, done: usize, total: usize) {
        if let Some(progress) = &self.progress {
            progress(done, total);
        }
    }

    fn assemble(cache: PathBuf, observer: Option<mc_dl::Observer>) -> Result<Self> {
        let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
        let dl = Arc::new(match observer {
            Some(observer) => dl.observe(observer),
            None => dl,
        });
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::new(dl.clone()),
            curseforge_web: crate::curseforge_web::CurseForgeWeb::new(dl.clone()),
            dl,
            cache,
            progress: None,
        })
    }

    /// A registry whose sources all point to a given root.
    ///
    /// Resolution is the most delicate thing this crate does — arbitration
    /// between requesters, catch-up of dependencies no API declares, stopping
    /// on cycles — and none of that can be verified without API responses.
    /// Against the real Modrinth, a test would depend on its availability and
    /// couldn't trigger any of the cases that matter.
    ///
    /// The two sources keep separate roots: they expose routes of the same
    /// shape, and conflating them would make it impossible to tell which one
    /// answered.
    #[cfg(test)]
    pub(crate) fn for_fixtures(cache: PathBuf, base: &str) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::with_base(dl.clone(), base),
            curseforge_web: crate::curseforge_web::CurseForgeWeb::with_bases(
                dl.clone(),
                &format!("{base}/web"),
                &format!("{base}/widget"),
            ),
            dl,
            cache,
            progress: None,
        })
    }
}
