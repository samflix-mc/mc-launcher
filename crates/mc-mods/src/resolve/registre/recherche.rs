//! Les candidats qu'une source propose pour une demande.

use anyhow::{Context, Result};

use crate::resolve::Registry;
use crate::resolve::demande::Request;
use crate::{Candidate, Origin};

use super::filtre::check_compatible;

impl Registry {
    /// Candidats pour un projet, dans la source demandée ou dans l'ordre par
    /// défaut.
    ///
    /// Un identifiant **numérique** ne peut venir que de CurseForge : le
    /// proposer à Modrinth ferait une requête vouée à l'échec pour chaque
    /// dépendance résolue.
    pub(crate) async fn candidates(
        &self,
        id_or_slug: &str,
        source: Option<Origin>,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let numeric = id_or_slug.parse::<u32>().is_ok();

        if source != Some(Origin::CurseForge) && !numeric {
            let found = self.modrinth.candidates(id_or_slug, mc, loader).await?;
            if !found.is_empty() || source == Some(Origin::Modrinth) {
                return Ok(found);
            }
        }
        self.curseforge_any(id_or_slug, mc, loader).await
    }
    pub(crate) async fn pinned(
        &self,
        request: &Request,
        mc: &str,
        loader: &str,
    ) -> Result<Option<Candidate>> {
        let Some(file) = &request.file else {
            return Ok(None);
        };
        // L'épinglage est explicite : si le build n'existe plus, il vaut mieux
        // s'arrêter que retomber en silence sur une autre version — c'est
        // précisément ce que l'épinglage sert à éviter.
        let numeric = request.slug.parse::<u32>().is_ok();
        let found = match request.source {
            Some(Origin::CurseForge) => self.curseforge_file(&request.slug, file).await?,
            Some(Origin::Modrinth) => self.modrinth.candidate_by_version(file).await?,
            None if numeric => self.curseforge_file(&request.slug, file).await?,
            None => match self.modrinth.candidate_by_version(file).await? {
                Some(found) => Some(found),
                None => self.curseforge_file(&request.slug, file).await?,
            },
        };

        let found = found
            .with_context(|| format!("build {file} épinglé pour {} : introuvable", request.slug))?;
        check_compatible(&found, mc, loader, &request.slug)?;
        Ok(Some(found))
    }
}

#[cfg(test)]
#[path = "recherche.test.rs"]
mod tests;
