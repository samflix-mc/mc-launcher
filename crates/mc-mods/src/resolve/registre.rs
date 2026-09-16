//! Les sources, interrogées dans l'ordre.

mod curseforge;
pub(super) mod filtre;
mod recherche;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Les deux sources, interrogées dans l'ordre.
pub struct Registry {
    pub(super) dl: Arc<mc_dl::Downloader>,
    pub(super) modrinth: crate::modrinth::Modrinth,
    pub(super) curseforge: Option<crate::curseforge::CurseForge>,
    pub(super) curseforge_web: crate::curseforge_web::CurseForgeWeb,
    /// Mémorise qu'une clé a été refusée, pour ne pas retenter — ni réavertir —
    /// à chacun des mods qui suivent.
    pub(super) key_rejected: std::sync::atomic::AtomicBool,
    pub(super) cache: PathBuf,
}

impl Registry {
    pub fn new(cache: PathBuf) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::new(dl.clone()),
            curseforge: crate::curseforge::CurseForge::from_env(dl.clone()),
            curseforge_web: crate::curseforge_web::CurseForgeWeb::new(dl.clone()),
            key_rejected: std::sync::atomic::AtomicBool::new(false),
            dl,
            cache,
        })
    }

    pub fn has_curseforge(&self) -> bool {
        self.curseforge.is_some()
    }
}
