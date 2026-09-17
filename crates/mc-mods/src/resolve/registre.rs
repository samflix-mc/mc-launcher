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
    pub(super) curseforge_web: crate::curseforge_web::CurseForgeWeb,
    pub(super) cache: PathBuf,
}

impl Registry {
    pub fn new(cache: PathBuf) -> Result<Self> {
        Self::monter(cache, None)
    }

    /// Le même registre, qui dit où en sont ses téléchargements.
    ///
    /// L'observateur se pose à la construction et non après coup : le client
    /// HTTP est partagé par les deux sources derrière un `Arc`, et il n'est
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
            curseforge_web: crate::curseforge_web::CurseForgeWeb::new(dl.clone()),
            dl,
            cache,
        })
    }

    /// Un registre dont toutes les sources pointent vers une racine donnée.
    ///
    /// La résolution est ce que ce crate fait de plus délicat — arbitrage entre
    /// demandeurs, rattrapage des dépendances qu'aucune API ne déclare, arrêt
    /// sur cycle — et rien de tout cela ne se vérifie sans réponses d'API.
    /// Contre le vrai Modrinth, un test dépendrait de sa disponibilité et ne
    /// pourrait provoquer aucun des cas qui comptent.
    ///
    /// Les deux sources gardent des racines distinctes : elles exposent des
    /// routes de même forme, et les confondre rendrait indiscernable celle qui
    /// a répondu.
    #[cfg(test)]
    pub(crate) fn pour_essais(cache: PathBuf, base: &str) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::avec_base(dl.clone(), base),
            curseforge_web: crate::curseforge_web::CurseForgeWeb::avec_bases(
                dl.clone(),
                &format!("{base}/web"),
                &format!("{base}/widget"),
            ),
            dl,
            cache,
        })
    }
}
