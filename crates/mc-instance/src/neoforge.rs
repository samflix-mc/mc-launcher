//! Installation de NeoForge, par son installateur officiel.
//!
//! Le descripteur de version de NeoForge ne suffit pas à installer le
//! chargeur : une partie des bibliothèques n'existe pas telle quelle sur un
//! dépôt Maven, elle est *fabriquée* au moment de l'installation par une suite
//! de traitements — application de patchs binaires au client vanilla,
//! découpage du jar, renommage des symboles. Ces traitements sont des jars
//! livrés avec l'installateur, et leur enchaînement change d'une version à
//! l'autre.
//!
//! Les réimplémenter reviendrait à suivre indéfiniment un format interne. On
//! exécute donc l'installateur publié, avec le Java que le launcher vient de
//! garantir. Il est idempotent, ce qui permet de le relancer sans risque.

use anyhow::{Context, Result, bail};
use mc_dl::{Check, Checksum, Downloader};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const MAVEN: &str = "https://maven.neoforged.net/releases";
const VERSIONS_API: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct VersionList {
    versions: Vec<String>,
}

/// Série NeoForge correspondant à une version de Minecraft.
///
/// NeoForge numérote `<majeur>.<mineur>.<correctif>` en reprenant les deux
/// premiers nombres de la version du jeu : Minecraft 1.21.1 donne la série
/// 21.1.x. C'est la seule correspondance à connaître, et elle est stable
/// depuis l'abandon du versionnage hérité de Forge.
pub fn series_for(mc: &str) -> Option<String> {
    let mut parts = mc.split('.');
    if parts.next()? != "1" {
        return None;
    }
    let major = parts.next()?;
    let minor = parts.next().unwrap_or("0");
    Some(format!("{major}.{minor}."))
}

/// Dernière version publiée de NeoForge pour une version de Minecraft.
pub async fn latest_for(mc: &str, dl: &Downloader) -> Result<String> {
    let series = series_for(mc)
        .with_context(|| format!("aucune série NeoForge ne correspond à Minecraft {mc}"))?;

    let list: VersionList = serde_json::from_slice(&dl.bytes(VERSIONS_API).await?)
        .context("liste des versions NeoForge illisible")?;

    // Les versions sont publiées dans l'ordre ; on trie tout de même par
    // numéro de correctif, une republication pouvant désordonner la liste.
    let mut matching: Vec<(u32, String)> = list
        .versions
        .into_iter()
        .filter(|v| v.starts_with(&series))
        .filter(|v| !v.contains("beta"))
        .filter_map(|v| {
            let patch = v.rsplit('.').next()?.parse().ok()?;
            Some((patch, v))
        })
        .collect();
    matching.sort_by_key(|(patch, _)| *patch);

    matching
        .pop()
        .map(|(_, v)| v)
        .with_context(|| format!("aucune version NeoForge {series}x publiée"))
}

fn installer_url(version: &str) -> String {
    format!("{MAVEN}/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar")
}

/// Télécharge l'installateur et vérifie son empreinte.
///
/// Le `.sha1` publié à côté de l'artefact est l'empreinte de référence du
/// dépôt. L'installateur est exécuté juste après : le vérifier n'est pas une
/// formalité.
async fn fetch_installer(version: &str, cache: &Path, dl: &Downloader) -> Result<PathBuf> {
    let url = installer_url(version);
    let dest = cache.join(format!("neoforge-{version}-installer.jar"));

    let sha1 = dl
        .bytes(&format!("{url}.sha1"))
        .await
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .with_context(|| format!("empreinte de l'installateur NeoForge {version}"))?;

    dl.to_file(&url, &dest, Check::Full(&Checksum::Sha1(sha1)))
        .await
        .with_context(|| format!("téléchargement de l'installateur NeoForge {version}"))?;
    Ok(dest)
}

/// Nom du répertoire de version produit par l'installateur.
pub fn version_id(version: &str) -> String {
    format!("neoforge-{version}")
}

/// Installe NeoForge côté client dans un répertoire de type `.minecraft`.
///
/// Les fichiers vanilla de la version visée doivent déjà s'y trouver :
/// l'installateur applique ses patchs au client de Mojang et échoue s'il ne le
/// trouve pas.
#[tracing::instrument(name = "neoforge client", skip(shared, cache, java, dl))]
pub async fn install_client(
    version: &str,
    shared: &Path,
    cache: &Path,
    java: &Path,
    dl: &Downloader,
) -> Result<PathBuf> {
    let produced = shared
        .join("versions")
        .join(version_id(version))
        .join(format!("{}.json", version_id(version)));
    if produced.is_file() {
        tracing::debug!(version, "déjà installé, installateur non relancé");
        return Ok(produced);
    }

    let installer = fetch_installer(version, cache, dl).await?;
    // L'installateur applique des patchs binaires : sur une machine lente c'est
    // une minute pendant laquelle rien ne bouge à l'écran.
    tracing::info!(
        version,
        "Exécution de l'installateur NeoForge {version} — peut prendre une minute"
    );

    // L'installateur refuse de démarrer sans ce fichier : il y inscrit un
    // profil pour le launcher officiel. On n'en fait rien, mais son absence
    // est une erreur fatale de son point de vue.
    let profiles = shared.join("launcher_profiles.json");
    if !profiles.is_file() {
        std::fs::create_dir_all(shared)?;
        std::fs::write(&profiles, br#"{"profiles":{},"version":3}"#)?;
    }

    run_installer(&installer, "--install-client", shared, java).await?;

    if !produced.is_file() {
        bail!(
            "l'installateur NeoForge {version} s'est terminé sans produire {}",
            produced.display()
        );
    }
    Ok(produced)
}

/// Installe un serveur NeoForge complet dans son propre répertoire.
#[tracing::instrument(name = "neoforge serveur", skip(dir, cache, java, dl))]
pub async fn install_server(
    version: &str,
    dir: &Path,
    cache: &Path,
    java: &Path,
    dl: &Downloader,
) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    // L'installateur va dans le cache partagé, pas dans le répertoire du
    // serveur : celui-ci est destiné à être recopié vers l'hôte qui fait
    // tourner le jeu, et n'a pas à emporter six mégaoctets d'outillage.
    let installer = fetch_installer(version, cache, dl).await?;
    run_installer(&installer, "--install-server", dir, java).await?;
    Ok(())
}

async fn run_installer(installer: &Path, mode: &str, dir: &Path, java: &Path) -> Result<()> {
    let output = tokio::process::Command::new(java)
        .arg("-jar")
        .arg(installer)
        .arg(mode)
        .arg(dir)
        .current_dir(dir)
        .output()
        .await
        .with_context(|| format!("exécution de {}", installer.display()))?;

    if !output.status.success() {
        // L'installateur écrit son diagnostic utile sur stdout et les traces
        // sur stderr ; les deux sont nécessaires pour comprendre un échec.
        bail!(
            "l'installateur NeoForge a échoué ({}) :\n{}\n{}",
            output.status,
            tail(&String::from_utf8_lossy(&output.stdout), 25),
            tail(&String::from_utf8_lossy(&output.stderr), 25)
        );
    }
    Ok(())
}

/// Dernières lignes d'une sortie, l'essentiel d'un échec s'y trouvant.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serie_deduite_de_la_version_du_jeu() {
        assert_eq!(series_for("1.21.1").as_deref(), Some("21.1."));
        assert_eq!(series_for("1.21").as_deref(), Some("21.0."));
        assert_eq!(series_for("1.20.4").as_deref(), Some("20.4."));
        // NeoForge ne couvre pas les versions antérieures au versionnage 1.x.
        assert_eq!(series_for("21w07a"), None);
    }

    #[test]
    fn identifiant_de_version_produit() {
        assert_eq!(version_id("21.1.250"), "neoforge-21.1.250");
    }

    #[test]
    fn url_de_l_installateur() {
        assert_eq!(
            installer_url("21.1.250"),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.250/neoforge-21.1.250-installer.jar"
        );
    }

    #[test]
    fn la_queue_garde_les_dernieres_lignes() {
        assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
        assert_eq!(tail("a", 5), "a");
    }
}
