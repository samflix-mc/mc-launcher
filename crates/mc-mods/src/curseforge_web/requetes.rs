//! Les builds visibles d'un projet, et ses dépendances déclarées.

use anyhow::Result;

use crate::{Candidate, DeclaredDep};

use super::api::{Page, WebDependency, WebFile};
use super::conversion::{compatible, to_candidate};
use super::{CurseForgeWeb, PAGE_SIZE};

impl CurseForgeWeb {
    /// Versions compatibles publiées par un projet.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let Some((project_id, name)) = self.resolve_project(id_or_slug).await? else {
            return Ok(Vec::new());
        };

        let Some(page): Option<Page<WebFile>> = self
            .get_json(&format!(
                "{}/mods/{project_id}/files?pageSize={PAGE_SIZE}&removeAlphas=false",
                self.web
            ))
            .await?
        else {
            return Ok(Vec::new());
        };

        let total = page.pagination.as_ref().map(|p| p.total_count).unwrap_or(0);
        let rendered = page.data.len();

        let slug = if id_or_slug.parse::<u32>().is_ok() {
            format!("{project_id}")
        } else {
            id_or_slug.to_string()
        };

        let found: Vec<Candidate> = page
            .data
            .into_iter()
            .filter(|f| compatible(&f.game_versions, mc, loader))
            .map(|f| to_candidate(&self.web, project_id, &slug, &name, f))
            .collect();

        // Rien trouvé alors que le projet publie bien plus que ce qu'on voit :
        // le silence serait trompeur, la version existe peut-être hors fenêtre.
        if found.is_empty() && total > rendered {
            anyhow::bail!(
                "{slug} : aucune version {mc}/{loader} parmi les {rendered} fichiers les plus \
                 récents, mais le projet en compte {total}. Sans clé d'API, CurseForge ne montre \
                 pas au-delà. Épingler le build avec « file », ou configurer CURSEFORGE_API_KEY."
            );
        }

        let mut found = found;
        if let Some(deps) = self.dependencies(project_id).await? {
            // Les dépendances du site sont déclarées par projet et non par
            // fichier : elles valent pour toutes les versions.
            for candidate in &mut found {
                candidate.declared_deps = deps.clone();
            }
        }
        Ok(found)
    }

    /// Dépendances obligatoires déclarées, à l'échelle du projet.
    pub(super) async fn dependencies(&self, project_id: u32) -> Result<Option<Vec<DeclaredDep>>> {
        let Some(page): Option<Page<WebDependency>> = self
            .get_json(&format!(
                "{}/mods/{project_id}/dependencies?pageSize=20",
                self.web
            ))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            page.data
                .into_iter()
                .filter(|d| d.kind == "RequiredDependency")
                .map(|d| DeclaredDep {
                    // Le slug est préféré à l'identifiant numérique : il permet
                    // de retrouver le projet sur Modrinth, qui publie les
                    // empreintes et la répartition client/serveur.
                    project_id: if d.slug.is_empty() {
                        d.id.to_string()
                    } else {
                        d.slug
                    },
                    version_id: None,
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
#[path = "requetes.test.rs"]
mod tests;
