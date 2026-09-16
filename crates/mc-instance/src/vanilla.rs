//! Les fichiers publiés par Mojang : client, bibliothèques, assets.
//!
//! Tout part de `version_manifest_v2.json`, qui renvoie vers le descripteur
//! d'une version, lequel décrit le reste. Chaque fichier vient avec son SHA-1,
//! ce qui permet de tout vérifier sans faire confiance au transport.
//!
//! Le partage est délibéré : bibliothèques et assets vivent dans un répertoire
//! commun à toutes les instances. Ils représentent près d'un gigaoctet, ne
//! dépendent que de la version du jeu, et les dupliquer par instance rendrait
//! inutilisable le fait d'en avoir plusieurs.

use anyhow::{Context, Result, bail};
use mc_dl::{Check, Checksum, Downloader};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const RESOURCES: &str = "https://resources.download.minecraft.net";

/// Téléchargements simultanés pour les assets.
///
/// Ce sont quelques milliers de fichiers de quelques kilooctets : la latence
/// domine, et la concurrence est ce qui fait la différence entre deux minutes
/// et une demi-heure.
const PARALLEL: usize = 16;

#[derive(Debug, Deserialize)]
struct Manifest {
    versions: Vec<ManifestVersion>,
}

#[derive(Debug, Deserialize)]
struct ManifestVersion {
    id: String,
    url: String,
    sha1: String,
}

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LibraryDownloads {
    pub(crate) artifact: Option<Artifact>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Library {
    pub(crate) name: String,
    pub(crate) downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub(crate) rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Rule {
    action: String,
    #[serde(default)]
    os: Option<OsCondition>,
    /// Drapeaux que le launcher active ou non — démo, résolution imposée,
    /// Quick Play. Absent des règles de bibliothèques, présent sur celles des
    /// arguments, d'où le même type pour les deux.
    #[serde(default)]
    features: Option<std::collections::BTreeMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OsCondition {
    name: Option<String>,
    arch: Option<String>,
}

/// Drapeaux actifs pour cette exécution, comparés aux `features` des règles.
pub(crate) type Features = std::collections::BTreeSet<String>;

#[derive(Debug, Deserialize)]
struct AssetIndexRef {
    id: String,
    sha1: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Downloads {
    client: Artifact,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionJson {
    id: String,
    main_class: String,
    asset_index: AssetIndexRef,
    downloads: Downloads,
    libraries: Vec<Library>,
    java_version: Option<JavaVersion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaVersion {
    major_version: u32,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: std::collections::BTreeMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

/// Ce que l'installation vanilla a produit.
#[derive(Debug)]
pub struct Vanilla {
    pub id: String,
    pub main_class: String,
    /// Version majeure de Java exigée par Mojang pour cette version du jeu.
    pub java_major: u32,
    pub client_jar: PathBuf,
    pub version_json: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub asset_index_id: String,
    pub assets_downloaded: usize,
}

/// Le nom d'une bibliothèque Maven vaut-il pour ce système ?
///
/// Les règles sont évaluées dans l'ordre, la dernière qui s'applique l'emporte.
/// En l'absence de toute règle, la bibliothèque est retenue — c'est le cas des
/// deux tiers d'entre elles.
pub(crate) fn allowed(rules: &[Rule], os: &str, arch: &str) -> bool {
    allowed_with(rules, os, arch, &Features::new())
}

/// Même évaluation, en tenant compte des drapeaux actifs.
///
/// Une règle conditionnée à un drapeau ne s'applique que si le launcher l'a
/// activé. C'est ce qui fait que `--quickPlayMultiplayer` n'apparaît sur la
/// ligne de commande que lorsqu'on demande effectivement de rejoindre un
/// serveur.
pub(crate) fn allowed_with(rules: &[Rule], os: &str, arch: &str, features: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allow = false;
    for rule in rules {
        let os_ok = match &rule.os {
            None => true,
            Some(condition) => {
                condition.name.as_deref().map(|n| n == os).unwrap_or(true)
                    && condition.arch.as_deref().map(|a| a == arch).unwrap_or(true)
            }
        };
        // Un drapeau demandé à `false` exige son absence : c'est ainsi que
        // Mojang exprime « sauf en démo ».
        let features_ok = match &rule.features {
            None => true,
            Some(wanted) => wanted
                .iter()
                .all(|(name, expected)| features.contains(name) == *expected),
        };
        if os_ok && features_ok {
            allow = rule.action == "allow";
        }
    }
    allow
}

/// Nom de l'OS dans le vocabulaire de Mojang.
pub fn mojang_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

pub fn mojang_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// Chemin Maven d'une bibliothèque dont Mojang ne publie pas le `path`.
///
/// `groupe:artefact:version[:classifier]` devient
/// `groupe/en/dossiers/artefact/version/artefact-version[-classifier].jar`.
/// Rare pour les bibliothèques vanilla, systématique pour celles que NeoForge
/// ajoute.
pub fn maven_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?.replace('.', "/");
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();

    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}

/// Installe les fichiers Mojang d'une version dans un répertoire partagé.
#[tracing::instrument(name = "jeu vanilla", skip(shared, dl))]
pub async fn install(mc: &str, shared: &Path, dl: &Downloader) -> Result<Vanilla> {
    let manifest: Manifest = serde_json::from_slice(&dl.bytes(MANIFEST).await?)
        .context("manifeste des versions illisible")?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == mc)
        .with_context(|| format!("Minecraft {mc} ne figure pas au manifeste de Mojang"))?;

    // Le descripteur est vérifié comme le reste : son SHA-1 figure dans le
    // manifeste, et c'est lui qui donne les empreintes de tous les autres
    // fichiers. Le corrompre reviendrait à corrompre l'installation entière.
    let version_json = shared
        .join("versions")
        .join(&entry.id)
        .join(format!("{}.json", entry.id));
    dl.to_file(
        &entry.url,
        &version_json,
        Check::Full(&Checksum::Sha1(entry.sha1.clone())),
    )
    .await
    .context("téléchargement du descripteur de version")?;

    let version: VersionJson = serde_json::from_slice(&std::fs::read(&version_json)?)
        .with_context(|| format!("{} illisible", version_json.display()))?;

    let client_jar = shared
        .join("versions")
        .join(&version.id)
        .join(format!("{}.jar", version.id));
    dl.to_file(
        &version.downloads.client.url,
        &client_jar,
        Check::Full(&Checksum::Sha1(version.downloads.client.sha1.clone())),
    )
    .await
    .context("téléchargement du client")?;

    tracing::debug!(
        version = %version.id,
        java_exige = version.java_version.as_ref().map(|j| j.major_version),
        bibliotheques_declarees = version.libraries.len(),
        "descripteur de version lu"
    );

    let libraries = install_libraries(&version, shared, dl).await?;
    let assets_downloaded = install_assets(&version.asset_index, shared, dl).await?;

    Ok(Vanilla {
        java_major: version.java_version.map(|j| j.major_version).unwrap_or(21),
        id: version.id,
        main_class: version.main_class,
        client_jar,
        version_json,
        libraries,
        asset_index_id: version.asset_index.id,
        assets_downloaded,
    })
}

#[tracing::instrument(name = "bibliothèques", skip_all)]
async fn install_libraries(
    version: &VersionJson,
    shared: &Path,
    dl: &Downloader,
) -> Result<Vec<PathBuf>> {
    use futures_util::stream::{self, StreamExt};

    let os = mojang_os();
    let arch = mojang_arch();
    let root = shared.join("libraries");

    let wanted: Vec<(String, Artifact)> = version
        .libraries
        .iter()
        .filter(|lib| allowed(&lib.rules, os, arch))
        .filter_map(|lib| {
            let artifact = lib.downloads.as_ref()?.artifact.as_ref()?;
            let path = artifact.path.clone().or_else(|| maven_path(&lib.name))?;
            Some((
                path,
                Artifact {
                    path: None,
                    sha1: artifact.sha1.clone(),
                    size: artifact.size,
                    url: artifact.url.clone(),
                },
            ))
        })
        .collect();

    let results: Vec<Result<PathBuf>> = stream::iter(wanted)
        .map(|(path, artifact)| {
            let dest = root.join(&path);
            async move {
                dl.to_file(
                    &artifact.url,
                    &dest,
                    Check::Full(&Checksum::Sha1(artifact.sha1.clone())),
                )
                .await
                .with_context(|| format!("bibliothèque {path}"))?;
                Ok(dest)
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    results.into_iter().collect()
}

#[tracing::instrument(name = "assets", skip_all, fields(index = %index.id))]
async fn install_assets(index: &AssetIndexRef, shared: &Path, dl: &Downloader) -> Result<usize> {
    use futures_util::stream::{self, StreamExt};

    let index_path = shared
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", index.id));
    dl.to_file(
        &index.url,
        &index_path,
        Check::Full(&Checksum::Sha1(index.sha1.clone())),
    )
    .await
    .context("téléchargement de l'index des assets")?;

    let parsed: AssetIndex = serde_json::from_slice(&std::fs::read(&index_path)?)
        .context("index des assets illisible")?;
    let objects = shared.join("assets").join("objects");

    let results: Vec<Result<mc_dl::Fetched>> = stream::iter(parsed.objects.into_values())
        .map(|object| {
            // Les objets sont adressés par leur empreinte : deux versions du
            // jeu partagent tout ce qui n'a pas changé.
            let prefix = &object.hash[..2];
            let dest = objects.join(prefix).join(&object.hash);
            let url = format!("{RESOURCES}/{prefix}/{}", object.hash);
            async move {
                let sum = Checksum::Sha1(object.hash.clone());
                dl.to_file(
                    &url,
                    &dest,
                    Check::Quick {
                        sum: &sum,
                        size: object.size,
                    },
                )
                .await
                .with_context(|| format!("asset {}", object.hash))
            }
        })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;

    let mut downloaded = 0;
    let total = results.len();
    for result in results {
        if result? == mc_dl::Fetched::Downloaded {
            downloaded += 1;
        }
    }
    // L'étape la plus longue d'une première installation, et la plus muette
    // d'une seconde : dire combien d'objets ont été passés explique pourquoi.
    tracing::info!(
        total,
        telecharges = downloaded,
        deja_presents = total - downloaded,
        "{downloaded} assets téléchargés sur {total} ({} déjà présents)",
        total - downloaded
    );
    Ok(downloaded)
}

/// Recontrôle l'empreinte de tous les objets d'assets déjà installés.
///
/// Complément de [`Check::Quick`] : l'installation ne compare que les tailles,
/// cette fonction fait le passage exhaustif quand on veut la certitude.
pub fn verify_assets(shared: &Path, index_id: &str) -> Result<VerifyReport> {
    let index_path = shared
        .join("assets")
        .join("indexes")
        .join(format!("{index_id}.json"));
    let parsed: AssetIndex = serde_json::from_slice(&std::fs::read(&index_path)?)
        .context("index des assets illisible")?;

    let objects = shared.join("assets").join("objects");
    let mut report = VerifyReport::default();
    for object in parsed.objects.values() {
        let path = objects.join(&object.hash[..2]).join(&object.hash);
        if !path.is_file() {
            report.missing.push(object.hash.clone());
            continue;
        }
        match mc_dl::sha1_of_file(&path) {
            Ok(sum) if sum.eq_ignore_ascii_case(&object.hash) => report.ok += 1,
            _ => report.corrupt.push(object.hash.clone()),
        }
    }
    Ok(report)
}

#[derive(Debug, Default)]
pub struct VerifyReport {
    pub ok: usize,
    pub missing: Vec<String>,
    pub corrupt: Vec<String>,
}

impl VerifyReport {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.corrupt.is_empty()
    }
}

/// Vue minimale d'un descripteur, limitée à ses bibliothèques.
///
/// Le descripteur que produit NeoForge ne porte ni `assetIndex` ni
/// `downloads` : il complète celui de la version qu'il désigne par
/// `inheritsFrom`. Le lire avec la structure complète échouerait, alors que
/// ses cinquante bibliothèques comptent autant que celles de Mojang.
#[derive(Debug, Deserialize)]
struct LibrariesOnly {
    libraries: Vec<Library>,
}

/// Bibliothèques retenues pour ce système, chemins relatifs au dépôt partagé.
///
/// S'applique indifféremment au descripteur de Mojang et à celui de NeoForge.
pub fn classpath(version_json: &Path, shared: &Path) -> Result<Vec<PathBuf>> {
    let version: LibrariesOnly = serde_json::from_slice(&std::fs::read(version_json)?)
        .with_context(|| format!("{} illisible", version_json.display()))?;
    let os = mojang_os();
    let arch = mojang_arch();
    let root = shared.join("libraries");

    let mut out = Vec::new();
    for lib in &version.libraries {
        if !allowed(&lib.rules, os, arch) {
            continue;
        }
        let Some(path) = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.path.clone())
            .or_else(|| maven_path(&lib.name))
        else {
            bail!("bibliothèque sans chemin exploitable : {}", lib.name);
        };
        out.push(root.join(path));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(action: &str, os: Option<&str>, arch: Option<&str>) -> Rule {
        Rule {
            action: action.to_string(),
            os: os.map(|name| OsCondition {
                name: Some(name.to_string()),
                arch: arch.map(str::to_string),
            }),
            features: None,
        }
    }

    /// Règle conditionnée à un drapeau, comme celles des arguments.
    fn feature_rule(action: &str, feature: &str, expected: bool) -> Rule {
        Rule {
            action: action.to_string(),
            os: None,
            features: Some(std::collections::BTreeMap::from([(
                feature.to_string(),
                expected,
            )])),
        }
    }

    #[test]
    fn sans_regle_la_bibliotheque_est_retenue() {
        assert!(allowed(&[], "linux", "x86_64"));
    }

    #[test]
    fn un_argument_sous_drapeau_n_apparait_que_si_le_drapeau_est_actif() {
        // C'est ainsi que --quickPlayMultiplayer reste absent tant qu'on ne
        // demande pas à rejoindre un serveur.
        let rules = vec![feature_rule("allow", "is_quick_play_multiplayer", true)];
        let actifs = Features::from(["is_quick_play_multiplayer".to_string()]);

        assert!(allowed_with(&rules, "linux", "x86_64", &actifs));
        assert!(!allowed_with(&rules, "linux", "x86_64", &Features::new()));
    }

    #[test]
    fn un_drapeau_attendu_faux_exige_son_absence() {
        // « sauf en démo » : la règle s'applique quand le drapeau est absent.
        let rules = vec![feature_rule("allow", "is_demo_user", false)];
        assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));

        let demo = Features::from(["is_demo_user".to_string()]);
        assert!(!allowed_with(&rules, "linux", "x86_64", &demo));
    }

    #[test]
    fn les_regles_de_bibliotheques_ignorent_les_drapeaux() {
        // Elles n'en portent pas : le comportement doit rester celui d'avant.
        let rules = vec![rule("allow", Some("linux"), None)];
        assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));
        assert!(!allowed_with(&rules, "osx", "x86_64", &Features::new()));
    }

    #[test]
    fn une_bibliotheque_reservee_a_macos_est_ecartee_ailleurs() {
        // java-objc-bridge n'a de sens que sur macOS ; l'installer sous Linux
        // alourdit le classpath sans jamais servir.
        let rules = vec![rule("allow", Some("osx"), None)];
        assert!(allowed(&rules, "osx", "x86_64"));
        assert!(!allowed(&rules, "linux", "x86_64"));
    }

    #[test]
    fn la_derniere_regle_applicable_l_emporte() {
        let rules = vec![
            rule("allow", None, None),
            rule("disallow", Some("osx"), None),
        ];
        assert!(allowed(&rules, "linux", "x86_64"));
        assert!(!allowed(&rules, "osx", "arm64"));
    }

    #[test]
    fn une_regle_peut_viser_une_architecture() {
        let rules = vec![rule("allow", Some("windows"), Some("x86"))];
        assert!(allowed(&rules, "windows", "x86"));
        assert!(!allowed(&rules, "windows", "x86_64"));
    }

    #[test]
    fn chemin_maven_avec_et_sans_classifier() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
        );
        assert_eq!(
            maven_path("net.neoforged:neoforge:21.1.250:client").unwrap(),
            "net/neoforged/neoforge/21.1.250/neoforge-21.1.250-client.jar"
        );
        assert!(maven_path("incomplet").is_none());
    }
}
