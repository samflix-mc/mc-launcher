//! Détection d'un runtime Java utilisable, sinon installation d'un Temurin.
//!
//! Minecraft 1.21.1 exige Java 21 : en dessous, le jeu s'arrête sur
//! `UnsupportedClassVersionError` avant même d'afficher une fenêtre. Un joueur
//! n'a aucune raison d'avoir un JDK, et celui qu'il a est souvent un 8 ou un 17
//! laissé par un vieux modpack.
//!
//! La stratégie est donc : chercher, vérifier, et n'installer qu'en dernier
//! recours — installer systématiquement coûterait 50 Mo à chaque poste pour
//! rien, et ne jamais installer renverrait l'utilisateur vers une page de
//! téléchargement, ce qui est précisément ce qu'un launcher doit éviter.
//!
//! Le runtime installé est **dédié au launcher** : il vit dans son répertoire
//! de données, n'est pas ajouté au `PATH`, et ne touche pas au Java du système.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Un runtime Java repéré ou installé, dont la version a été **mesurée** en
/// exécutant le binaire — jamais déduite de son chemin.
#[derive(Debug, Clone)]
pub struct Java {
    /// Exécutable `java` (`java.exe` sous Windows).
    pub path: PathBuf,
    pub version: Version,
    pub origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Installé par le launcher dans son propre répertoire.
    Managed,
    /// Trouvé sur le système (`JAVA_HOME`, `PATH`, emplacements usuels).
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    /// Chaîne complète telle que rapportée, p. ex. `21.0.5+11`.
    pub full: String,
}

/// Extrait le numéro majeur d'une chaîne de version Java.
///
/// Deux schémas coexistent encore : `1.8.0_412` (jusqu'à Java 8, où le majeur
/// est le *deuxième* nombre) et `21.0.5` (depuis Java 9). Confondre les deux
/// ferait passer un Java 8 pour un Java 1.
pub fn parse_major(version: &str) -> Option<u32> {
    let cleaned: String = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .to_string();
    let mut parts = cleaned.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Interroge un exécutable `java` et lit sa version.
///
/// `-version` écrit sur **stderr** (choix historique de la JVM) et sur trois
/// lignes dont seule la première porte le numéro, entre guillemets.
pub async fn probe(exe: &Path) -> Result<Version> {
    let out = tokio::process::Command::new(exe)
        .arg("-version")
        .output()
        .await
        .with_context(|| format!("exécution de {}", exe.display()))?;

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let quoted = text
        .split('"')
        .nth(1)
        .with_context(|| format!("version illisible dans la sortie de {}", exe.display()))?;
    let major = parse_major(quoted)
        .with_context(|| format!("numéro majeur illisible dans « {quoted} »"))?;

    Ok(Version {
        major,
        full: quoted.to_string(),
    })
}

/// Répertoire où le launcher installe ses runtimes Java.
pub fn default_runtime_dir() -> PathBuf {
    mc_dl::data_dir().join("runtime")
}

/// Emplacement du runtime géré pour une version majeure donnée.
pub fn managed_home(runtime_dir: &Path, major: u32) -> PathBuf {
    runtime_dir.join(format!("temurin-{major}"))
}

fn java_exe(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join("bin").join("java.exe")
    } else {
        home.join("bin").join("java")
    }
}

/// Candidats à tester, du plus fiable au plus douteux.
///
/// Le runtime géré passe en premier : s'il est là, c'est le launcher qui l'a
/// installé et vérifié, inutile de sonder le système.
fn candidates(runtime_dir: &Path, major: u32) -> Vec<PathBuf> {
    let mut found = vec![java_exe(&managed_home(runtime_dir, major))];

    if let Some(home) = std::env::var_os("JAVA_HOME") {
        found.push(java_exe(Path::new(&home)));
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let exe = dir.join(if cfg!(windows) { "java.exe" } else { "java" });
            if exe.is_file() {
                found.push(exe);
            }
        }
    }

    // Emplacements usuels des paquets système. Les distributions installent
    // plusieurs JDK côte à côte et n'en exposent qu'un dans le `PATH` ; le 21
    // demandé est souvent présent mais non par défaut.
    let roots: &[&str] = if cfg!(target_os = "macos") {
        &["/Library/Java/JavaVirtualMachines"]
    } else if cfg!(windows) {
        &[
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
        ]
    } else {
        &["/usr/lib/jvm", "/usr/lib64/jvm", "/opt/java"]
    };
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let home = entry.path();
            // macOS empaquette le JDK dans un bundle.
            found.push(java_exe(&home.join("Contents").join("Home")));
            found.push(java_exe(&home));
        }
    }

    found
}

/// Premier runtime trouvé dont la version majeure est **au moins** `major`.
///
/// Un Java 22 fait tourner du bytecode compilé pour 21 ; refuser une version
/// supérieure obligerait à réinstaller à chaque montée de version du système.
/// L'inverse est impossible, d'où la comparaison asymétrique.
pub async fn detect(major: u32, runtime_dir: &Path) -> Option<Java> {
    let managed = java_exe(&managed_home(runtime_dir, major));
    let mut seen = Vec::new();

    for exe in candidates(runtime_dir, major) {
        if !exe.is_file() {
            continue;
        }
        let real = std::fs::canonicalize(&exe).unwrap_or_else(|_| exe.clone());
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);

        // Un exécutable peut être présent et cassé (paquet à moitié
        // désinstallé, lien symbolique mort) : on ne retient que ce qui répond.
        let Ok(version) = probe(&exe).await else {
            continue;
        };
        if version.major >= major {
            let origin = if exe == managed {
                Origin::Managed
            } else {
                Origin::System
            };
            return Some(Java {
                path: exe,
                version,
                origin,
            });
        }
    }
    None
}

/// Garantit la présence d'un Java ≥ `major` : détection, sinon installation.
#[tracing::instrument(name = "runtime java", skip(runtime_dir))]
pub async fn ensure(major: u32, runtime_dir: &Path) -> Result<Java> {
    if let Some(java) = detect(major, runtime_dir).await {
        tracing::debug!(
            version = %java.version.full,
            chemin = %java.path.display(),
            "runtime existant retenu"
        );
        return Ok(java);
    }
    // Le seul cas qui coûte du temps et de la bande passante : il mérite d'être
    // visible sans avoir à relever la verbosité.
    tracing::info!(
        majeur = major,
        "Aucun Java {major} sur ce poste, installation de Temurin"
    );
    install(major, runtime_dir).await
}

// --- API Adoptium -----------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Asset {
    binary: Binary,
    release_name: String,
}

#[derive(Debug, Deserialize)]
struct Binary {
    package: Package,
    image_type: String,
}

#[derive(Debug, Deserialize)]
struct Package {
    link: String,
    name: String,
    checksum: String,
}

/// Couple `(os, architecture)` au vocabulaire d'Adoptium.
fn platform() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        "windows" => "windows",
        other => bail!("système {other} non couvert par les binaires Temurin"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => bail!("architecture {other} non couverte par les binaires Temurin"),
    };
    Ok((os, arch))
}

/// Télécharge et installe Temurin dans `runtime_dir`.
///
/// Un **JRE** est demandé en premier : il pèse la moitié d'un JDK et suffit à
/// tout ce que fait le launcher, y compris aux *processors* de l'installateur
/// NeoForge, qui sont des jars. Adoptium ne publie pas de JRE pour toutes les
/// combinaisons de plateformes, d'où le repli sur le JDK.
#[tracing::instrument(name = "installation java", skip(runtime_dir))]
pub async fn install(major: u32, runtime_dir: &Path) -> Result<Java> {
    let (os, arch) = platform()?;
    std::fs::create_dir_all(runtime_dir)
        .with_context(|| format!("création de {}", runtime_dir.display()))?;
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    let mut asset = None;
    for image in ["jre", "jdk"] {
        let url = format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot\
             ?architecture={arch}&image_type={image}&os={os}&vendor=eclipse"
        );
        let body = dl.bytes(&url).await?;
        let assets: Vec<Asset> = serde_json::from_slice(&body)
            .with_context(|| format!("réponse Adoptium illisible pour {image} {major}"))?;
        if let Some(found) = assets.into_iter().find(|a| a.binary.image_type == image) {
            asset = Some(found);
            break;
        }
    }
    let asset = asset
        .with_context(|| format!("Adoptium ne publie pas de Java {major} pour {os}/{arch}"))?;

    let home = managed_home(runtime_dir, major);
    let archive = runtime_dir.join(&asset.binary.package.name);
    // Adoptium publie un SHA-256 par paquet : un JDK est du code exécuté avec
    // les droits de l'utilisateur, le vérifier n'est pas optionnel.
    let sum = mc_dl::Checksum::Sha256(asset.binary.package.checksum.clone());
    dl.to_file(
        &asset.binary.package.link,
        &archive,
        mc_dl::Check::Full(&sum),
    )
    .await
    .with_context(|| format!("téléchargement de {}", asset.release_name))?;

    if home.exists() {
        std::fs::remove_dir_all(&home)?;
    }
    // Extraction dans un répertoire temporaire : l'archive contient un dossier
    // racine au nom de la version, qu'on ne veut pas dans le chemin final.
    let staging = runtime_dir.join(format!(".temurin-{major}-extraction"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    extract(&archive, &staging)?;

    let root = single_child(&staging)?;
    std::fs::rename(&root, &home)
        .with_context(|| format!("installation vers {}", home.display()))?;
    std::fs::remove_dir_all(&staging).ok();
    std::fs::remove_file(&archive).ok();

    // Dernière vérification, et la seule qui prouve quoi que ce soit : le
    // binaire installé démarre et annonce la bonne version.
    let exe = java_exe(&home);
    let version = probe(&exe)
        .await
        .with_context(|| format!("le Java installé dans {} ne démarre pas", home.display()))?;
    if version.major < major {
        bail!(
            "Temurin {} installé, mais il annonce Java {} alors que {major} est exigé",
            asset.release_name,
            version.major
        );
    }

    Ok(Java {
        path: exe,
        version,
        origin: Origin::Managed,
    })
}

/// Unique entrée d'un répertoire — le dossier racine de l'archive Temurin.
fn single_child(dir: &Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    match entries.len() {
        1 => Ok(entries.remove(0)),
        n => bail!(
            "archive Temurin inattendue : {n} entrées à la racine de {}",
            dir.display()
        ),
    }
}

fn extract(archive: &Path, into: &Path) -> Result<()> {
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    if name.ends_with(".zip") {
        extract_zip(archive, into)
    } else if name.ends_with(".tar.gz") {
        let file = std::fs::File::open(archive)?;
        tar::Archive::new(flate2::read::GzDecoder::new(file))
            .unpack(into)
            .with_context(|| format!("extraction de {name}"))?;
        Ok(())
    } else {
        bail!("format d'archive non géré : {name}")
    }
}

/// Extraction ZIP, pour la variante Windows d'Adoptium.
///
/// Les entrées sont validées avant écriture : une archive peut contenir des
/// chemins remontants (`../`) qui écriraient hors du répertoire cible.
fn extract_zip(archive: &Path, into: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let Some(relative) = entry.enclosed_name() else {
            bail!("entrée d'archive au chemin suspect : {}", entry.name());
        };
        let dest = into.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn majeur_des_deux_schemas_de_version() {
        assert_eq!(parse_major("21.0.5+11"), Some(21));
        assert_eq!(parse_major("21"), Some(21));
        assert_eq!(parse_major("17.0.9"), Some(17));
        // Jusqu'à Java 8, le majeur est le deuxième nombre.
        assert_eq!(parse_major("1.8.0_412"), Some(8));
        assert_eq!(parse_major("1.7.0_80"), Some(7));
        assert_eq!(parse_major("22-ea"), Some(22));
        assert_eq!(parse_major(""), None);
    }

    #[test]
    fn le_runtime_gere_est_le_premier_candidat() {
        let dir = Path::new("/tmp/mc-runtime");
        let list = candidates(dir, 21);
        assert_eq!(list[0], managed_home(dir, 21).join("bin").join("java"));
    }
}
