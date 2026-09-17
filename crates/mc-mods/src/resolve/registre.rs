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
        Self::monter(cache, None)
    }

    /// Le même registre, qui dit où en sont ses téléchargements.
    ///
    /// L'observateur se pose à la construction et non après coup : le client
    /// HTTP est partagé par les trois sources derrière un `Arc`, et il n'est
    /// donc plus modifiable une fois le registre monté.
    pub fn observee(cache: PathBuf, observateur: mc_dl::Observateur) -> Result<Self> {
        Self::monter(cache, Some(observateur))
    }

    fn monter(cache: PathBuf, observateur: Option<mc_dl::Observateur>) -> Result<Self> {
        let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
        let dl = Arc::new(match observateur {
            Some(observateur) => dl.observe(observateur),
            None => dl,
        });
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

    /// Un registre dont toutes les sources pointent vers une racine donnée.
    ///
    /// La résolution est ce que ce crate fait de plus délicat — arbitrage entre
    /// demandeurs, rattrapage des dépendances qu'aucune API ne déclare, arrêt
    /// sur cycle — et rien de tout cela ne se vérifie sans réponses d'API.
    /// Contre le vrai Modrinth, un test dépendrait de sa disponibilité et ne
    /// pourrait provoquer aucun des cas qui comptent.
    #[cfg(test)]
    pub(crate) fn pour_essais(cache: PathBuf, base: &str, cle: Option<&str>) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::avec_base(dl.clone(), base),
            curseforge: cle
                .map(|cle| crate::curseforge::CurseForge::avec_base(dl.clone(), cle.into(), base)),
            // Racines distinctes : la Core API et le site exposent la même
            // route `/mods/{id}/files` sous des formes différentes, et les
            // confondre rendrait indiscernable la source qui a répondu.
            curseforge_web: crate::curseforge_web::CurseForgeWeb::avec_bases(
                dl.clone(),
                &format!("{base}/web"),
                &format!("{base}/widget"),
            ),
            key_rejected: std::sync::atomic::AtomicBool::new(false),
            dl,
            cache,
        })
    }
}
