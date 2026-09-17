//! Les builds qu'un projet propose, et celui qu'on épingle.

use anyhow::{Context, Result};

use crate::Candidate;

use super::CurseForge;
use super::api::{ApiFile, Envelope};
use super::conversion::{loader_type, to_candidate};

impl CurseForge {
    /// Versions compatibles, désignées par slug ou par identifiant numérique.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let project = match id_or_slug.parse::<u32>() {
            Ok(id) => self.project_by_id(id).await?,
            Err(_) => self.project_by_slug(id_or_slug).await?,
        };
        let Some(project) = project else {
            return Ok(Vec::new());
        };

        let query = [
            ("gameVersion", mc.to_string()),
            ("modLoaderType", loader_type(loader).to_string()),
            ("pageSize", "50".to_string()),
        ];
        let files: Option<Envelope<Vec<ApiFile>>> = self
            .get_json(&self.url(&format!("/mods/{}/files", project.id)), &query)
            .await?;
        let files = files.map(|e| e.data).unwrap_or_default();

        Ok(files
            .into_iter()
            .filter_map(|f| to_candidate(&project, f))
            .collect())
    }

    /// Fichier précis, pour un build épinglé dans le manifeste.
    pub async fn candidate_by_file(&self, file_id: &str) -> Result<Option<Candidate>> {
        let file_id: u32 = file_id.parse().with_context(|| {
            format!("identifiant de fichier CurseForge non numérique : {file_id}")
        })?;

        // La route d'un fichier isolé exige aussi l'identifiant du projet ; on
        // passe donc par la recherche par empreinte de fichier, qui ne
        // l'exige pas.
        let found: Option<Envelope<Vec<ApiFile>>> = self
            .post_json(
                &self.url("/mods/files"),
                serde_json::json!({ "fileIds": [file_id] }),
            )
            .await?;
        let Some(file) = found.and_then(|e| e.data.into_iter().next()) else {
            return Ok(None);
        };
        let Some(project) = self.project_by_id(file.mod_id).await? else {
            return Ok(None);
        };
        Ok(to_candidate(&project, file))
    }
}
