//! Recontrôler ce qui est déjà installé, et dire ce qu'il faut à la JVM.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::descripteur::{AssetIndex, Library};
use super::plateforme::{maven_path, mojang_arch, mojang_os};
use super::regles::allowed;

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
