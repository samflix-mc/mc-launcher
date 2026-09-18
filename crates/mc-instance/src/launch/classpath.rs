//! The libraries the JVM must find, in the order it must find them.

use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::vanilla;

use super::descriptor::{VersionJson, library_key};

/// Walked from most specific to most general: the loader first, Mojang
/// after. The first occurrence of a library wins, so a version replaced by
/// the loader takes the place of the game's.
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
                bail!("library with no usable path: {}", library.name);
            };
            seen.insert(key, library.name.clone());
            classpath.push(libraries_root.join(relative));
        }
    }

    // Mojang's client only joins the classpath for pure vanilla.
    //
    // Under a loader, the installer has produced its own split of the
    // client — `client-…-slim.jar` for the code, `client-…-extra.jar` for
    // the resources — and FML resolves them itself from `libraryDirectory`.
    // Adding `1.21.1.jar` on top yields two modules exporting the same
    // packages, and the JVM stops before the first screen:
    //
    //     java.lang.module.ResolutionException: Modules _1._21._1 and
    //     minecraft export package com.mojang.blaze3d.systems to module …
    //
    // The name `_1._21._1` is the one the JVM derives from `1.21.1.jar`: it
    // unambiguously points to the jar added here, and that's what made it
    // possible to trace the cause.
    let base = chain.last().expect("at least one version");
    let uses_loader = chain.len() > 1;
    let client_jar = shared
        .join("versions")
        .join(&base.id)
        .join(format!("{}.jar", base.id));
    if !client_jar.is_file() {
        bail!("missing client: {}", client_jar.display());
    }
    if uses_loader {
        tracing::debug!(
            client = %client_jar.display(),
            "vanilla client left out of the classpath, the loader supplies its own"
        );
    } else {
        classpath.push(client_jar);
    }

    let missing: Vec<&PathBuf> = classpath.iter().filter(|p| !p.is_file()).collect();
    if let Some(first) = missing.first() {
        bail!(
            "{} missing libraries, starting with {} — reinstall",
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

#[cfg(test)]
#[path = "classpath.test.rs"]
mod tests;
