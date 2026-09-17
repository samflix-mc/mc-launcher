//! CurseForge, par l'API publique de son site.
//!
//! La Core API — celle qui demande une clé d'inscription — n'est plus
//! interrogée. Ce qui reste est l'API que le site sert à ses propres pages,
//! complétée par cfwidget pour retrouver un identifiant de projet à partir
//! d'un slug.
//!
//! Ce que ce choix coûte est dit dans `docs/mods.md`, et ce n'est pas rien :
//! pas d'empreinte publiée, pas de recherche par mot-clé, et plus aucun moyen
//! de connaître `allowModDistribution`.

use anyhow::Result;

use crate::Candidate;
use crate::resolve::Registry;

impl Registry {
    /// Candidats pour un projet.
    pub(crate) async fn curseforge_any(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        self.curseforge_web.candidates(id_or_slug, mc, loader).await
    }

    /// Cherche un projet par le `modId` que déclare un jar.
    ///
    /// Modrinth d'abord : sa recherche est ouverte et rend le bon projet même
    /// quand le `modId` ne ressemble pas au slug. CurseForge ensuite, où la
    /// recherche par mot-clé est fermée — seul un `modId` qui est aussi le
    /// slug du projet peut aboutir.
    pub(crate) async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let found = self.modrinth.find_by_mod_id(mod_id, mc, loader).await?;
        if !found.is_empty() {
            return Ok(found);
        }
        // Un échec n'est pas une absence de résultat, mais ici il revient au
        // même : c'est la dernière source, et la boucle de rattrapage doit
        // pouvoir conclure que ce `modId` n'a pas de projet plutôt que de tout
        // arrêter.
        self.curseforge_web
            .find_by_mod_id(mod_id, mc, loader)
            .await
            .or_else(|_| Ok(Vec::new()))
    }

    /// Build CurseForge épinglé par le verrou.
    pub(crate) async fn curseforge_file(
        &self,
        id_or_slug: &str,
        file: &str,
    ) -> Result<Option<Candidate>> {
        self.curseforge_web
            .candidate_by_file(id_or_slug, file)
            .await
    }
}

#[cfg(test)]
#[path = "curseforge.test.rs"]
mod tests;
