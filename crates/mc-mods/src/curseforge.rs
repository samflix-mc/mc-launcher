//! Source CurseForge — consultée quand Modrinth ne connaît pas le projet.
//!
//! Elle vient en second pour une raison pratique : son API exige une clé,
//! nominative et à demander sur console.curseforge.com. Le launcher ne peut pas
//! en embarquer une — elle serait extraite du binaire et révoquée — donc
//! l'installation ne doit dépendre de CurseForge que lorsque c'est inévitable,
//! c'est-à-dire pour les mods qui n'y sont pas publiés ailleurs.
//!
//! Deux limites de la source, assumées ici plutôt que contournées :
//!
//! - elle ne dit pas si un mod est client, serveur ou les deux. Le côté est
//!   donc `both` par défaut, affiné ensuite par ce que déclare le jar ;
//! - un auteur peut interdire le téléchargement par un tiers
//!   (`downloadUrl: null`). Reconstruire l'URL du CDN contournerait ce refus :
//!   le cas est signalé avec le lien de la page, et le fichier attendu dans le
//!   dossier des apports manuels.

use crate::jar::Side;
use crate::{Candidate, Channel, DeclaredDep, Origin};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::sync::Arc;

const API: &str = "https://api.curseforge.com/v1";
/// Minecraft dans la nomenclature CurseForge.
const GAME_MINECRAFT: u32 = 432;
/// Classe « Mods », par opposition aux modpacks et aux ressource packs.
const CLASS_MODS: u32 = 6;
/// `modLoaderType` de NeoForge.
const LOADER_NEOFORGE: u32 = 6;
const LOADER_FORGE: u32 = 1;
const LOADER_FABRIC: u32 = 4;
const LOADER_QUILT: u32 = 5;

/// `relationType` d'une dépendance obligatoire.
const RELATION_REQUIRED: u32 = 3;

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMod {
    id: u32,
    name: String,
    slug: String,
    links: Links,
    #[serde(default)]
    allow_mod_distribution: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiFile {
    id: u32,
    mod_id: u32,
    display_name: String,
    file_name: String,
    release_type: u32,
    file_date: String,
    download_url: Option<String>,
    file_length: u64,
    #[serde(default)]
    hashes: Vec<ApiHash>,
    #[serde(default)]
    dependencies: Vec<ApiDependency>,
}

#[derive(Debug, Deserialize)]
struct ApiHash {
    value: String,
    algo: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiDependency {
    mod_id: u32,
    relation_type: u32,
}

pub struct CurseForge {
    dl: Arc<mc_dl::Downloader>,
    key: String,
}

/// Clé d'API, cherchée dans l'environnement puis dans la configuration.
///
/// Le fichier permet de ne pas exporter la clé dans chaque shell, et de ne pas
/// la voir passer dans l'historique des commandes.
pub fn api_key() -> Option<String> {
    if let Ok(key) = std::env::var("CURSEFORGE_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }
    let path = config_key_path();
    let key = std::fs::read_to_string(path).ok()?.trim().to_string();
    (!key.is_empty()).then_some(key)
}

pub fn config_key_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("samflix-mc").join("curseforge.key")
}

impl CurseForge {
    pub fn new(dl: Arc<mc_dl::Downloader>, key: String) -> Self {
        Self { dl, key }
    }

    /// Construit la source si une clé est disponible, sinon `None`.
    pub fn from_env(dl: Arc<mc_dl::Downloader>) -> Option<Self> {
        api_key().map(|key| Self::new(dl, key))
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
            .header("x-api-key", &self.key)
            .query(query)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => return Ok(None),
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED => {
                bail!(
                    "CurseForge refuse la clé d'API (HTTP {}). \
                     Vérifier CURSEFORGE_API_KEY ou {}",
                    response.status(),
                    config_key_path().display()
                );
            }
            _ => {}
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("GET {url}"))?;
        Ok(Some(
            response
                .json()
                .await
                .with_context(|| format!("réponse de {url}"))?,
        ))
    }

    async fn project_by_slug(&self, slug: &str) -> Result<Option<ApiMod>> {
        let query = [
            ("gameId", GAME_MINECRAFT.to_string()),
            ("classId", CLASS_MODS.to_string()),
            ("slug", slug.to_string()),
        ];
        let found: Option<Envelope<Vec<ApiMod>>> =
            self.get_json(&format!("{API}/mods/search"), &query).await?;
        Ok(found.and_then(|e| e.data.into_iter().next()))
    }

    async fn project_by_id(&self, id: u32) -> Result<Option<ApiMod>> {
        let found: Option<Envelope<ApiMod>> =
            self.get_json(&format!("{API}/mods/{id}"), &[]).await?;
        Ok(found.map(|e| e.data))
    }

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
            .get_json(&format!("{API}/mods/{}/files", project.id), &query)
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
                &format!("{API}/mods/files"),
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

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .post(url)
            .header("x-api-key", &self.key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("POST {url}"))?;
        Ok(Some(response.json().await?))
    }

    /// Cherche le projet portant un `modId`, pour le rattrapage des
    /// dépendances qu'aucune API ne déclare.
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

        let query = [
            ("gameId", GAME_MINECRAFT.to_string()),
            ("classId", CLASS_MODS.to_string()),
            ("searchFilter", mod_id.to_string()),
            ("gameVersion", mc.to_string()),
            ("modLoaderType", loader_type(loader).to_string()),
            ("pageSize", "5".to_string()),
        ];
        let found: Option<Envelope<Vec<ApiMod>>> =
            self.get_json(&format!("{API}/mods/search"), &query).await?;
        let Some(hits) = found else {
            return Ok(Vec::new());
        };

        for hit in hits.data {
            let candidates = self.candidates(&hit.id.to_string(), mc, loader).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }
}

fn loader_type(loader: &str) -> u32 {
    match loader.to_ascii_lowercase().as_str() {
        "neoforge" => LOADER_NEOFORGE,
        "forge" => LOADER_FORGE,
        "fabric" => LOADER_FABRIC,
        "quilt" => LOADER_QUILT,
        _ => LOADER_NEOFORGE,
    }
}

/// `releaseType` : 1 = release, 2 = beta, 3 = alpha.
fn channel_of(release_type: u32) -> Channel {
    match release_type {
        1 => Channel::Release,
        2 => Channel::Beta,
        _ => Channel::Alpha,
    }
}

fn to_candidate(project: &ApiMod, file: ApiFile) -> Option<Candidate> {
    // `algo` : 1 = SHA-1, 2 = MD5. Le SHA-1 est préféré ; les fichiers anciens
    // n'ont parfois qu'un MD5, auquel cas le fichier est téléchargé sans
    // vérification et le lockfile le consigne.
    let sha1 = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.clone());

    let declared = file
        .dependencies
        .iter()
        .filter(|d| d.relation_type == RELATION_REQUIRED)
        .map(|d| DeclaredDep {
            project_id: d.mod_id.to_string(),
            version_id: None,
        })
        .collect();

    Some(Candidate {
        origin: Origin::CurseForge,
        project_id: project.id.to_string(),
        slug: project.slug.clone(),
        name: project.name.clone(),
        version_id: file.id.to_string(),
        version_number: file.display_name.clone(),
        display_name: file.display_name,
        channel: channel_of(file.release_type),
        file_name: file.file_name,
        // Un `downloadUrl` absent traduit le refus de l'auteur d'être
        // redistribué : l'URL n'est pas reconstruite, l'absence est propagée et
        // deviendra un message explicite au moment du téléchargement.
        url: file.download_url.unwrap_or_default(),
        sha1,
        size: file.file_length,
        published: file.file_date,
        // CurseForge ne publie pas la répartition client/serveur ; le côté sera
        // affiné par le descripteur du jar.
        project_side: Side::Both,
        declared_deps: declared,
        // Conservée pour que le refus de redistribution donne un message
        // actionnable — la page du mod — plutôt qu'un « introuvable ».
        page_url: project.links.website_url.clone(),
        redistributable: project.allow_mod_distribution.unwrap_or(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiants_de_chargeur() {
        assert_eq!(loader_type("neoforge"), 6);
        assert_eq!(loader_type("NeoForge"), 6);
        assert_eq!(loader_type("fabric"), 4);
    }

    #[test]
    fn canaux_de_publication() {
        assert_eq!(channel_of(1), Channel::Release);
        assert_eq!(channel_of(2), Channel::Beta);
        assert_eq!(channel_of(3), Channel::Alpha);
    }
}
