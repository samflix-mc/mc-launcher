//! CurseForge, via the public API of its own site.
//!
//! The Core API (`api.curseforge.com`) requires a nominative key: a launcher
//! cannot embed one — it would be extracted from the binary and revoked — and
//! requiring one from every player amounts to asking them for a developer
//! account just to install a modpack. So it is no longer queried at all.
//!
//! The site itself serves its own pages through an API that asks for nothing:
//! that's what's borrowed here, completed by cfwidget for the one thing it
//! refuses, the mapping between a slug and a project id.
//!
//! It is therefore the second and last source, after Modrinth. What's lost in
//! the process, and worth keeping in mind:
//!
//! - **no digest.** Only the file size is published. The SHA-1 is therefore
//!   computed on first download and frozen into the lockfile: subsequent
//!   installs are verified normally, only the very first one isn't;
//! - **no `allowModDistribution`.** Only the Core API exposes this flag, by
//!   which an author refuses to be downloaded automatically by a third-party
//!   launcher. It's no longer readable. The download still goes through the
//!   site's route and never through a reconstructed CDN URL — it's that
//!   reconstruction that would actively bypass a refusal — but the refusal
//!   itself now escapes us;
//! - **keyword search closed.** Only an exact slug, as it appears in the
//!   mod's page address, resolves a project;
//! - **fifty visible files.** Pagination is ignored by the server and
//!   `pageSize` is capped. A mod that has published more than fifty files
//!   since its last compatible version becomes invisible — the case is
//!   detected and reported rather than rendered as "missing";
//! - **none of this is contractual.** These routes serve the website, are
//!   undocumented, and cfwidget is a volunteer third-party service. Both can
//!   change without notice, unlike Modrinth.
//!
//! All these routes **wrap their response in `data`**, the single object just
//! like the list. Reading an object without its envelope produces a
//! deserialization failure, and a pinned build that's genuinely there gets
//! declared missing.

mod api;
mod conversion;
mod pinned;
mod project;
mod requests;

use std::sync::Arc;

pub(crate) const WEB: &str = "https://www.curseforge.com/api/v1";
pub(crate) const WIDGET: &str = "https://api.cfwidget.com/minecraft/mc-mods";

/// Cap imposed by the server, no matter what's asked.
pub(crate) const PAGE_SIZE: usize = 50;

/// The keyless client: the same routes as the site itself.
pub struct CurseForgeWeb {
    pub(crate) dl: Arc<mc_dl::Downloader>,
    /// Both roots are substitutable for tests. This is where it matters
    /// most: these routes are undocumented and cfwidget is a volunteer
    /// third-party service, so none of what follows can be verified against
    /// the real service without making it responsible for CI.
    pub(crate) web: String,
    pub(crate) widget: String,
}

impl CurseForgeWeb {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self::with_bases(dl, WEB, WIDGET)
    }

    pub(crate) fn with_bases(dl: Arc<mc_dl::Downloader>, web: &str, widget: &str) -> Self {
        Self {
            dl,
            web: web.to_string(),
            widget: widget.to_string(),
        }
    }
}
