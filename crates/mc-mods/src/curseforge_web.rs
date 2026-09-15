//! CurseForge sans clé — dernier recours, quand tout le reste a échoué.
//!
//! La Core API (`api.curseforge.com`) exige une clé nominative. Le site web,
//! lui, sert ses propres pages avec une API qui n'en demande pas : c'est elle
//! qu'on emprunte ici, complétée par cfwidget pour la seule chose qu'elle
//! refuse, la correspondance entre un slug et un identifiant de projet.
//!
//! Ce chemin n'est tenté qu'en troisième position — après Modrinth, après la
//! Core API si une clé existe — et il faut savoir ce qu'on y perd :
//!
//! - **pas d'empreinte.** Seule la taille du fichier est publiée. Le SHA-1 est
//!   donc calculé au premier téléchargement et figé dans le verrou : les
//!   installations suivantes sont vérifiées normalement, seule la toute
//!   première ne l'est pas ;
//! - **pas de `allowModDistribution`.** La Core API expose ce drapeau, par
//!   lequel un auteur refuse d'être téléchargé automatiquement par un launcher
//!   tiers. Il est absent de ces routes : ce mode ne peut pas l'honorer, et
//!   c'est la raison pour laquelle il passe en dernier plutôt qu'en premier ;
//! - **cinquante fichiers visibles.** La pagination est ignorée par le serveur
//!   et `pageSize` est plafonné. Un mod qui a publié plus de cinquante fichiers
//!   depuis sa dernière version compatible devient invisible — le cas est
//!   détecté et signalé plutôt que rendu comme « introuvable » ;
//! - **rien de tout cela n'est contractuel.** Ces routes servent le site web,
//!   ne sont pas documentées, et cfwidget est un service tiers bénévole. Les
//!   deux peuvent changer sans préavis, contrairement à Modrinth.

use crate::jar::Side;
use crate::{Candidate, Channel, DeclaredDep, Origin};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::Arc;

const WEB: &str = "https://www.curseforge.com/api/v1";
const WIDGET: &str = "https://api.cfwidget.com/minecraft/mc-mods";

/// Plafond imposé par le serveur, quoi qu'on demande.
const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize)]
struct Page<T> {
    data: Vec<T>,
    #[serde(default)]
    pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
struct Pagination {
    #[serde(rename = "totalCount")]
    total_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebFile {
    id: u64,
    file_name: String,
    display_name: String,
    file_length: u64,
    release_type: u32,
    date_created: String,
    #[serde(default)]
    game_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebDependency {
    id: u32,
    slug: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct Widget {
    id: u32,
    title: String,
}

pub struct CurseForgeWeb {
    dl: Arc<mc_dl::Downloader>,
}

impl CurseForgeWeb {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self { dl }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        // 403 est la réponse de Cloudflare comme celle d'une route fermée :
        // dans les deux cas, cette source n'a rien à offrir, et l'appelant doit
        // pouvoir continuer sans que tout s'arrête.
        if !response.status().is_success() {
            return Ok(None);
        }
        Ok(response.json().await.ok())
    }

    /// Identifiant de projet à partir d'un slug.
    ///
    /// La recherche du site répond 403 ; cfwidget est la seule voie restante.
    /// Il répond 202 le temps de constituer son cache pour un projet qu'il n'a
    /// jamais vu, d'où la seconde tentative.
    async fn project_id(&self, slug: &str) -> Result<Option<(u32, String)>> {
        for attempt in 0..2 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            let response = self
                .dl
                .client()
                .get(format!("{WIDGET}/{slug}"))
                .send()
                .await
                .with_context(|| format!("cfwidget pour {slug}"))?;

            if response.status() == reqwest::StatusCode::ACCEPTED {
                continue;
            }
            if !response.status().is_success() {
                return Ok(None);
            }
            let widget: Widget = match response.json().await {
                Ok(w) => w,
                Err(_) => return Ok(None),
            };
            return Ok(Some((widget.id, widget.title)));
        }
        Ok(None)
    }

    /// Résout un slug ou un identifiant numérique en `(id, nom)`.
    async fn resolve_project(&self, id_or_slug: &str) -> Result<Option<(u32, String)>> {
        match id_or_slug.parse::<u32>() {
            // Un identifiant numérique vient d'une dépendance déjà résolue : le
            // nom lisible n'est pas indispensable, et l'économiser évite un
            // appel à un service tiers.
            Ok(id) => Ok(Some((id, format!("projet {id}")))),
            Err(_) => self.project_id(id_or_slug).await,
        }
    }

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
                "{WEB}/mods/{project_id}/files?pageSize={PAGE_SIZE}&removeAlphas=false"
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
            .map(|f| to_candidate(project_id, &slug, &name, f))
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
    async fn dependencies(&self, project_id: u32) -> Result<Option<Vec<DeclaredDep>>> {
        let Some(page): Option<Page<WebDependency>> = self
            .get_json(&format!("{WEB}/mods/{project_id}/dependencies?pageSize=20"))
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

    /// Build précis, pour un épinglage du manifeste.
    pub async fn candidate_by_file(
        &self,
        id_or_slug: &str,
        file_id: &str,
    ) -> Result<Option<Candidate>> {
        let Some((project_id, name)) = self.resolve_project(id_or_slug).await? else {
            return Ok(None);
        };
        let Some(file): Option<WebFile> = self
            .get_json(&format!("{WEB}/mods/{project_id}/files/{file_id}"))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(to_candidate(project_id, id_or_slug, &name, file)))
    }

    /// Cherche le projet portant un `modId`, la recherche par mot-clé étant
    /// fermée. Ne fonctionne donc que lorsque le `modId` est aussi le slug.
    pub async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        self.candidates(mod_id, mc, loader).await
    }
}

/// Chargeurs que CurseForge nomme dans `gameVersions`.
const LOADERS: &[&str] = &["neoforge", "forge", "fabric", "quilt"];

/// Un fichier convient-il à cette version du jeu et à ce chargeur ?
///
/// `gameVersions` mélange versions, chargeurs et côtés — p. ex.
/// `["1.21", "Client", "1.21.1", "NeoForge", "Server"]`. Quand aucun chargeur
/// n'y figure, le fichier est accepté : c'est le cas des mods anciens, publiés
/// avant que CurseForge ne l'étiquette.
fn compatible(game_versions: &[String], mc: &str, loader: &str) -> bool {
    let lower: Vec<String> = game_versions
        .iter()
        .map(|v| v.to_ascii_lowercase())
        .collect();

    if !lower.iter().any(|v| v == mc) {
        return false;
    }
    let declares_loader = lower.iter().any(|v| LOADERS.contains(&v.as_str()));
    !declares_loader || lower.iter().any(|v| v == loader)
}

fn channel_of(release_type: u32) -> Channel {
    match release_type {
        1 => Channel::Release,
        2 => Channel::Beta,
        _ => Channel::Alpha,
    }
}

/// URL de téléchargement du site, celle qu'emprunte son propre bouton.
///
/// L'URL du CDN n'est pas reconstruite à partir de l'identifiant : c'est par
/// cette reconstruction qu'on contournerait le refus d'un auteur d'être
/// redistribué. Passer par la route du site laisse CurseForge décider.
fn download_url(project_id: u32, file_id: u64) -> String {
    format!("{WEB}/mods/{project_id}/files/{file_id}/download")
}

fn to_candidate(project_id: u32, slug: &str, name: &str, file: WebFile) -> Candidate {
    Candidate {
        origin: Origin::CurseForge,
        project_id: project_id.to_string(),
        slug: slug.to_string(),
        name: name.to_string(),
        version_id: file.id.to_string(),
        version_number: file.display_name.clone(),
        display_name: file.display_name,
        channel: channel_of(file.release_type),
        url: download_url(project_id, file.id),
        file_name: file.file_name,
        // Aucune empreinte publiée par cette source : le SHA-1 sera calculé au
        // téléchargement puis figé dans le verrou.
        sha1: None,
        size: file.file_length,
        published: file.date_created,
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: Some(format!(
            "https://www.curseforge.com/minecraft/mc-mods/{slug}"
        )),
        redistributable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn compatibilite_lue_dans_game_versions() {
        // Forme réelle renvoyée par le site pour JEI.
        let jei = versions(&["1.21", "Client", "1.21.1", "NeoForge", "Server"]);
        assert!(compatible(&jei, "1.21.1", "neoforge"));
        assert!(!compatible(&jei, "1.21.1", "fabric"));
        assert!(!compatible(&jei, "1.20.1", "neoforge"));
    }

    #[test]
    fn un_fichier_sans_chargeur_declare_est_accepte() {
        // Mods publiés avant que CurseForge n'étiquette le chargeur : les
        // écarter reviendrait à les rendre introuvables.
        let vieux = versions(&["1.21.1", "Client"]);
        assert!(compatible(&vieux, "1.21.1", "neoforge"));
    }

    #[test]
    fn le_bon_chargeur_est_exige_quand_il_est_declare() {
        let fabric = versions(&["1.21.1", "Fabric"]);
        assert!(!compatible(&fabric, "1.21.1", "neoforge"));
        assert!(compatible(&fabric, "1.21.1", "fabric"));
    }

    #[test]
    fn l_url_passe_par_la_route_du_site() {
        // Et non par une URL de CDN reconstruite, qui contournerait le refus
        // éventuel de l'auteur.
        let url = download_url(238222, 8886909);
        assert!(url.starts_with("https://www.curseforge.com/api/v1/"));
        assert!(url.ends_with("/mods/238222/files/8886909/download"));
        assert!(!url.contains("forgecdn"));
    }

    #[test]
    fn canaux() {
        assert_eq!(channel_of(1), Channel::Release);
        assert_eq!(channel_of(2), Channel::Beta);
        assert_eq!(channel_of(3), Channel::Alpha);
    }
}
