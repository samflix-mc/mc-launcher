//! Les bibliothèques que la JVM doit trouver, dans l'ordre où elle doit les
//! trouver.

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::vanilla;

use super::descripteur::{library_key, VersionJson};

/// Parcours du plus spécifique au plus général : le chargeur d'abord, Mojang
/// ensuite. La première occurrence d'une bibliothèque gagne, donc une version
/// remplacée par le chargeur prend la place de celle du jeu.
pub(super) fn classpath(
    chain: &[VersionJson],
    shared: &Path,
    os: &str,
    arch: &str,
    separator: &str,
) -> Result<(Vec<PathBuf>, String)> {
    let libraries_root = shared.join("libraries");
    let mut seen = BTreeMap::new();
    let mut classpath: Vec<PathBuf> = Vec::new();
    for version in chain {
        for library in &version.libraries {
            if !vanilla::allowed(&library.rules, os, arch) {
                continue;
            }
            let key = library_key(&library.name);
            if seen.contains_key(&key) {
                continue;
            }
            let Some(relative) = library
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.clone())
                .or_else(|| vanilla::maven_path(&library.name))
            else {
                bail!("bibliothèque sans chemin exploitable : {}", library.name);
            };
            seen.insert(key, library.name.clone());
            classpath.push(libraries_root.join(relative));
        }
    }

    // Le client de Mojang ne rejoint le classpath que pour du vanilla pur.
    //
    // Sous un chargeur, l'installateur a produit sa propre découpe du client —
    // `client-…-slim.jar` pour le code, `client-…-extra.jar` pour les
    // ressources — et FML les résout lui-même à partir de `libraryDirectory`.
    // Ajouter `1.21.1.jar` par-dessus donne deux modules qui exportent les
    // mêmes paquets, et la JVM s'arrête avant le premier écran :
    //
    //     java.lang.module.ResolutionException: Modules _1._21._1 and
    //     minecraft export package com.mojang.blaze3d.systems to module …
    //
    // Le nom `_1._21._1` est celui que la JVM dérive de `1.21.1.jar` : il
    // désigne sans ambiguïté le jar ajouté ici, et c'est ce qui a permis de
    // retrouver la cause.
    let base = chain.last().expect("au moins une version");
    let uses_loader = chain.len() > 1;
    let client_jar = shared
        .join("versions")
        .join(&base.id)
        .join(format!("{}.jar", base.id));
    if !client_jar.is_file() {
        bail!("client absent : {}", client_jar.display());
    }
    if uses_loader {
        tracing::debug!(
            client = %client_jar.display(),
            "client vanilla laissé hors du classpath, le chargeur fournit le sien"
        );
    } else {
        classpath.push(client_jar);
    }

    let missing: Vec<&PathBuf> = classpath.iter().filter(|p| !p.is_file()).collect();
    if let Some(first) = missing.first() {
        bail!(
            "{} bibliothèques manquantes, à commencer par {} — relancer l'installation",
            missing.len(),
            first.display()
        );
    }

    let classpath_text = classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(separator);
    Ok((classpath, classpath_text))
}
