//! Source Modrinth — consultée en premier, sans clé d'API.
//!
//! Modrinth est prioritaire pour trois raisons concrètes : son API est ouverte
//! (aucune clé à distribuer avec le launcher), elle publie le SHA-1 et le
//! SHA-512 de chaque fichier, et elle expose `client_side` / `server_side` par
//! projet — ce qui donne la répartition client/serveur sans avoir à la saisir
//! à la main dans le manifeste.
//!
//! Elle demande en revanche un `User-Agent` identifiable et applique une limite
//! de débit ; les appels par lots sont donc préférés aux boucles de requêtes.

use crate::{Candidate, Channel, DeclaredDep, Origin};
use crate::jar::Side;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::Arc;

const API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Deserialize)]
struct Project {
    id: String,
    slug: String,
    title: String,
    client_side: String,
    server_side: String,
}

#[derive(Debug, Deserialize)]
struct ApiVersion {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    version_type: String,
    date_published: String,
    files: Vec<ApiFile>,
    dependencies: Vec<ApiDependency>,
}

#[derive(Debug, Deserialize)]
struct ApiFile {
    url: String,
    filename: String,
    primary: bool,
    size: u64,
    hashes: ApiHashes,
}

#[derive(Debug, Deserialize)]
struct ApiHashes {
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiDependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    slug: String,
}

pub struct Modrinth {
    dl: Arc<mc_dl::Downloader>,
}

impl Modrinth {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self { dl }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .query(query)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        // Un projet absent de Modrinth est un cas nominal : c'est ce qui
        // déclenche le repli sur CurseForge, pas une erreur à remonter.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response.error_for_status().with_context(|| format!("GET {url}"))?;
        Ok(Some(response.json().await.with_context(|| format!("réponse de {url}"))?))
    }

    /// Versions publiées d'un projet, compatibles avec `mc` et `loader`.
    ///
    /// `id_or_slug` accepte indifféremment l'identifiant court (`u6dRKJwZ`) ou
    /// le slug (`jei`) : Modrinth résout les deux sur la même route, ce qui
    /// permet de traiter une dépendance — désignée par identifiant — comme une
    /// demande du manifeste, désignée par slug.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let Some(project): Option<Project> =
            self.get_json(&format!("{API}/project/{id_or_slug}"), &[]).await?
        else {
            return Ok(Vec::new());
        };

        let query = [
            ("loaders", format!("[\"{loader}\"]")),
            ("game_versions", format!("[\"{mc}\"]")),
        ];
        let versions: Vec<ApiVersion> = self
            .get_json(&format!("{API}/project/{}/version", project.id), &query)
            .await?
            .unwrap_or_default();

        Ok(versions
            .into_iter()
            .filter_map(|v| to_candidate(&project, v))
            .collect())
    }

    /// Version précise, pour un build épinglé dans le manifeste.
    pub async fn candidate_by_version(&self, version_id: &str) -> Result<Option<Candidate>> {
        let Some(version): Option<ApiVersion> =
            self.get_json(&format!("{API}/version/{version_id}"), &[]).await?
        else {
            return Ok(None);
        };
        let Some(project): Option<Project> = self
            .get_json(&format!("{API}/project/{}", version.project_id), &[])
            .await?
        else {
            return Ok(None);
        };
        Ok(to_candidate(&project, version))
    }

    /// Cherche le projet qui porte un `modId` donné.
    ///
    /// Sert au rattrapage des dépendances qu'aucune API ne déclare : le jar dit
    /// « il me faut `bookshelf` », et il faut retrouver le projet
    /// correspondant. Le slug coïncide souvent avec le `modId`, mais pas
    /// toujours — `bookshelf` est publié sous `bookshelf-lib` — d'où la
    /// recherche en second recours.
    pub async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let direct = self.candidates(mod_id, mc, loader).await?;
        if !direct.is_empty() {
            return Ok(direct);
        }

        let facets = format!(
            "[[\"project_type:mod\"],[\"categories:{loader}\"],[\"versions:{mc}\"]]"
        );
        let query = [
            ("query", mod_id.to_string()),
            ("facets", facets),
            ("limit", "5".to_string()),
        ];
        let Some(found): Option<SearchResponse> =
            self.get_json(&format!("{API}/search"), &query).await?
        else {
            return Ok(Vec::new());
        };

        // Les résultats sont classés par pertinence ; on s'arrête au premier
        // projet qui a effectivement une version compatible.
        for hit in found.hits {
            let candidates = self.candidates(&hit.slug, mc, loader).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }
}

/// Traduit le couple `client_side` / `server_side` en un côté d'installation.
///
/// `unsupported` d'un côté est la seule information tranchée : partout
/// ailleurs, `optional` domine et le mod est installé des deux côtés. C'est le
/// choix prudent avec NeoForge, qui compare les registres à la connexion et
/// refuse un client dont la liste de mods diffère de celle du serveur.
fn side_of(project: &Project) -> Side {
    let client = project.client_side != "unsupported";
    let server = project.server_side != "unsupported";
    match (client, server) {
        (true, false) => Side::Client,
        (false, true) => Side::Server,
        _ => Side::Both,
    }
}

fn to_candidate(project: &Project, version: ApiVersion) -> Option<Candidate> {
    // Une version porte parfois plusieurs fichiers (sources, variantes) ; le
    // fichier « primary » est celui que le launcher doit installer.
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())?;

    let declared = version
        .dependencies
        .iter()
        .filter(|d| d.dependency_type == "required")
        .filter_map(|d| {
            Some(DeclaredDep {
                project_id: d.project_id.clone()?,
                version_id: d.version_id.clone(),
            })
        })
        .collect();

    Some(Candidate {
        origin: Origin::Modrinth,
        project_id: project.id.clone(),
        slug: project.slug.clone(),
        name: project.title.clone(),
        version_id: version.id,
        version_number: version.version_number,
        display_name: version.name,
        channel: Channel::parse(&version.version_type),
        file_name: file.filename.clone(),
        url: file.url.clone(),
        sha1: file.hashes.sha1.clone(),
        size: file.size,
        published: version.date_published,
        project_side: side_of(project),
        declared_deps: declared,
        page_url: Some(format!("https://modrinth.com/mod/{}", project.slug)),
        redistributable: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(client: &str, server: &str) -> Project {
        Project {
            id: "id".into(),
            slug: "slug".into(),
            title: "Titre".into(),
            client_side: client.into(),
            server_side: server.into(),
        }
    }

    #[test]
    fn cote_deduit_des_metadonnees_du_projet() {
        assert_eq!(side_of(&project("required", "unsupported")), Side::Client);
        assert_eq!(side_of(&project("unsupported", "required")), Side::Server);
        assert_eq!(side_of(&project("required", "required")), Side::Both);
        // JEI et Jade sont « optional / optional » : installés des deux côtés,
        // faute de quoi les registres NeoForge divergeraient.
        assert_eq!(side_of(&project("optional", "optional")), Side::Both);
    }
}
