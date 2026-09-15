//! Ce qu'un jar déclare vraiment, lu dans le jar et non dans une API.
//!
//! Les dépendances annoncées par Modrinth ou CurseForge sont saisies à la main
//! par l'auteur au moment de la publication. Elles sont souvent incomplètes :
//! une bibliothèque ajoutée entre deux versions, une dépendance considérée
//! comme évidente, un envoi automatisé qui ne remplit pas le champ. Le jeu, lui,
//! ne lit que `META-INF/neoforge.mods.toml` — et s'arrête au démarrage dès
//! qu'une dépendance obligatoire y manque.
//!
//! On lit donc la même source que NeoForge. Deux détails décident de la
//! justesse du résultat :
//!
//! - **JarJar** : un mod peut embarquer ses bibliothèques dans
//!   `META-INF/jarjar/`. Elles fournissent leur `modId` sans exister comme
//!   fichier séparé. Les ignorer ferait conclure à une dépendance manquante et
//!   installerait un doublon — deux versions du même mod, ce que NeoForge
//!   refuse ;
//! - **le `side` d'une dépendance** : une dépendance déclarée `side = "CLIENT"`
//!   n'a rien à faire dans le dossier `mods` du serveur.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::path::Path;

/// Côté sur lequel un mod ou une dépendance a un sens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Client,
    Server,
    Both,
}

impl Side {
    /// Côté couvrant les deux usages : un mod tiré côté client *et* côté
    /// serveur doit finir dans les deux dossiers.
    pub fn union(self, other: Side) -> Side {
        if self == other { self } else { Side::Both }
    }

    pub fn includes(self, other: Side) -> bool {
        self == Side::Both || self == other
    }

    pub fn parse(text: &str) -> Option<Side> {
        match text.trim().to_ascii_uppercase().as_str() {
            "CLIENT" => Some(Side::Client),
            "SERVER" | "DEDICATED_SERVER" => Some(Side::Server),
            "BOTH" => Some(Side::Both),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Side::Client => "client",
            Side::Server => "server",
            Side::Both => "both",
        }
    }
}

/// Identifiants que NeoForge considère comme toujours présents : ils décrivent
/// la plateforme, pas un mod à installer.
pub const PLATFORM_IDS: &[&str] = &["minecraft", "neoforge", "forge", "java", "fml", "mcp"];

pub fn is_platform(mod_id: &str) -> bool {
    PLATFORM_IDS.contains(&mod_id.to_ascii_lowercase().as_str())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `modId` exigé, tel qu'écrit dans le descripteur.
    pub mod_id: String,
    /// Intervalle Maven, p. ex. `[21.1.65,)`. Conservé pour le diagnostic.
    pub version_range: Option<String>,
    pub side: Side,
}

/// Contenu utile d'un jar de mod.
#[derive(Debug, Clone, Default)]
pub struct JarInfo {
    /// `modId` que ce jar fournit, y compris ceux de ses jars embarqués.
    pub provides: BTreeSet<String>,
    /// Dépendances obligatoires, hors plateforme.
    pub requires: Vec<Requirement>,
}

/// Lit un jar sur le disque.
pub fn inspect(path: &Path) -> Result<JarInfo> {
    let bytes = std::fs::read(path).with_context(|| format!("lecture de {}", path.display()))?;
    inspect_bytes(&bytes).with_context(|| format!("analyse de {}", path.display()))
}

/// Lit un jar déjà en mémoire, en descendant dans ses jars embarqués.
pub fn inspect_bytes(bytes: &[u8]) -> Result<JarInfo> {
    let mut info = JarInfo::default();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;

    // NeoForge 1.20.5+ utilise neoforge.mods.toml ; mods.toml reste lu pour les
    // jars publiés avant le renommage, encore nombreux en 1.21.
    let descriptor = read_entry(&mut archive, "META-INF/neoforge.mods.toml")
        .or_else(|| read_entry(&mut archive, "META-INF/mods.toml"));

    if let Some(text) = descriptor {
        let parsed = parse_descriptor(&String::from_utf8_lossy(&text))?;
        info.provides.extend(parsed.provides);
        info.requires.extend(parsed.requires);
    }

    for nested in embedded_jars(&mut archive)? {
        // Seuls les `modId` fournis comptent : les dépendances d'une
        // bibliothèque embarquée sont, par construction, satisfaites par le
        // mod qui l'embarque.
        if let Ok(sub) = inspect_bytes(&nested) {
            info.provides.extend(sub.provides);
        }
    }

    Ok(info)
}

fn read_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    let mut buffer = Vec::new();
    entry.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

/// Jars embarqués par JarJar, listés dans `META-INF/jarjar/metadata.json`.
fn embedded_jars<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<Vec<Vec<u8>>> {
    let Some(raw) = read_entry(archive, "META-INF/jarjar/metadata.json") else {
        return Ok(Vec::new());
    };
    let metadata: serde_json::Value = serde_json::from_slice(&raw)?;
    let Some(jars) = metadata.get("jars").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for jar in jars {
        let Some(path) = jar.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(bytes) = read_entry(archive, path) {
            out.push(bytes);
        }
    }
    Ok(out)
}

/// Analyse un `neoforge.mods.toml`.
///
/// La table `dependencies` est indexée par le `modId` du mod déclarant, ce qui
/// permet à un jar multi-mods d'avoir des dépendances distinctes par mod. On ne
/// distingue pas les déclarants : ce qui compte est l'union de ce que le jar
/// exige pour démarrer.
pub fn parse_descriptor(text: &str) -> Result<JarInfo> {
    let root: toml::Value = text.parse().context("neoforge.mods.toml illisible")?;
    let mut info = JarInfo::default();

    if let Some(mods) = root.get("mods").and_then(|v| v.as_array()) {
        for entry in mods {
            if let Some(id) = entry.get("modId").and_then(|v| v.as_str()) {
                info.provides.insert(id.to_string());
            }
        }
    }

    let Some(dependencies) = root.get("dependencies").and_then(|v| v.as_table()) else {
        return Ok(info);
    };

    for declarations in dependencies.values() {
        let Some(list) = declarations.as_array() else {
            continue;
        };
        for dep in list {
            let Some(id) = dep.get("modId").and_then(|v| v.as_str()) else {
                continue;
            };
            if is_platform(id) {
                continue;
            }
            if !is_mandatory(dep) {
                continue;
            }
            let side = dep
                .get("side")
                .and_then(|v| v.as_str())
                .and_then(Side::parse)
                .unwrap_or(Side::Both);
            let requirement = Requirement {
                mod_id: id.to_string(),
                version_range: dep
                    .get("versionRange")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                side,
            };
            // Un jar multi-mods exige souvent la même bibliothèque plusieurs
            // fois ; on fusionne les côtés plutôt que de dupliquer.
            match info
                .requires
                .iter_mut()
                .find(|r| r.mod_id == requirement.mod_id)
            {
                Some(existing) => existing.side = existing.side.union(requirement.side),
                None => info.requires.push(requirement),
            }
        }
    }

    Ok(info)
}

/// Une dépendance est-elle obligatoire ?
///
/// NeoForge 1.21 écrit `type = "required"`. Les jars antérieurs, et ceux portés
/// depuis Forge, écrivent `mandatory = true`. Les deux formes se croisent dans
/// un même modpack.
fn is_mandatory(dep: &toml::Value) -> bool {
    if let Some(kind) = dep.get("type").and_then(|v| v.as_str()) {
        return kind.eq_ignore_ascii_case("required");
    }
    dep.get("mandatory")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descripteur_neoforge_moderne() {
        let info = parse_descriptor(
            r#"
modLoader="javafml"
loaderVersion="[21,)"
license="MIT"

[[mods]]
modId="attributefix"
version="21.1.3"

[[dependencies.attributefix]]
modId="neoforge"
type="required"
versionRange="[21.1.65,)"

[[dependencies.attributefix]]
modId="bookshelf"
type="required"
versionRange="[21.1.0,)"

[[dependencies.attributefix]]
modId="prickle"
type="optional"
"#,
        )
        .unwrap();

        assert!(info.provides.contains("attributefix"));
        // neoforge est la plateforme, prickle est facultatif : reste bookshelf.
        assert_eq!(info.requires.len(), 1);
        assert_eq!(info.requires[0].mod_id, "bookshelf");
        assert_eq!(info.requires[0].version_range.as_deref(), Some("[21.1.0,)"));
    }

    #[test]
    fn descripteur_forge_avec_mandatory() {
        let info = parse_descriptor(
            r#"
[[mods]]
modId="vieuxmod"

[[dependencies.vieuxmod]]
modId="jei"
mandatory=true

[[dependencies.vieuxmod]]
modId="jade"
mandatory=false
"#,
        )
        .unwrap();
        assert_eq!(info.requires.len(), 1);
        assert_eq!(info.requires[0].mod_id, "jei");
    }

    #[test]
    fn le_side_d_une_dependance_est_retenu() {
        let info = parse_descriptor(
            r#"
[[mods]]
modId="skin"

[[dependencies.skin]]
modId="embeddium"
type="required"
side="CLIENT"
"#,
        )
        .unwrap();
        assert_eq!(info.requires[0].side, Side::Client);
    }

    #[test]
    fn un_jar_multi_mods_fusionne_les_cotes_d_une_meme_dependance() {
        let info = parse_descriptor(
            r#"
[[mods]]
modId="a"
[[mods]]
modId="b"

[[dependencies.a]]
modId="lib"
type="required"
side="CLIENT"

[[dependencies.b]]
modId="lib"
type="required"
side="SERVER"
"#,
        )
        .unwrap();
        assert_eq!(info.provides.len(), 2);
        assert_eq!(info.requires.len(), 1);
        assert_eq!(info.requires[0].side, Side::Both);
    }

    #[test]
    fn union_des_cotes() {
        assert_eq!(Side::Client.union(Side::Client), Side::Client);
        assert_eq!(Side::Client.union(Side::Server), Side::Both);
        assert_eq!(Side::Both.union(Side::Client), Side::Both);
        assert!(Side::Both.includes(Side::Server));
        assert!(!Side::Client.includes(Side::Server));
    }
}
