//! L'assemblage : de la chaîne de versions à la ligne de commande.

use anyhow::{Context, Result};
use std::path::Path;

use crate::vanilla;

use super::arguments::assembler;
use super::classpath::classpath;
use super::commande::Command;
use super::descripteur::resolve_chain;
use super::session::{LaunchOptions, Session};
use super::variables::{active_features, variables};

#[tracing::instrument(
    name = "ligne de commande",
    skip(shared, game_dir, java, session, options),
    fields(version = version_id)
)]
pub fn build(
    version_id: &str,
    shared: &Path,
    game_dir: &Path,
    java: &Path,
    session: &Session,
    options: &LaunchOptions,
) -> Result<Command> {
    let chain = resolve_chain(shared, version_id)?;
    let base = chain.last().expect("au moins une version");

    let main_class = chain
        .iter()
        .find_map(|v| v.main_class.clone())
        .context("aucune classe principale dans la chaîne de versions")?;
    let assets_index = chain
        .iter()
        .find_map(|v| v.asset_index.as_ref().map(|a| a.id.clone()))
        .or_else(|| chain.iter().find_map(|v| v.assets.clone()))
        .context("aucun index d'assets dans la chaîne de versions")?;

    let os = vanilla::mojang_os();
    let arch = vanilla::mojang_arch();
    let features = active_features(options);
    let separator = if cfg!(windows) { ";" } else { ":" };

    let (classpath, classpath_text) = classpath(&chain, shared, os, arch, separator)?;

    // LWJGL y dépose les binaires qu'il sort des jars ; il refuse de démarrer
    // si le répertoire n'existe pas.
    let natives = shared.join("natives").join(&base.id);
    std::fs::create_dir_all(&natives)
        .with_context(|| format!("création de {}", natives.display()))?;
    std::fs::create_dir_all(game_dir)
        .with_context(|| format!("création de {}", game_dir.display()))?;

    let variables = variables(
        version_id,
        &base.id,
        game_dir,
        shared,
        &assets_index,
        &natives,
        &shared.join("libraries"),
        &classpath_text,
        separator,
        session,
        options,
    );

    let args = assembler(
        &chain,
        os,
        arch,
        &features,
        &variables,
        &natives,
        &classpath_text,
        main_class,
        options,
    );

    tracing::debug!(
        bibliotheques = classpath.len(),
        arguments = args.len(),
        "ligne de commande assemblée"
    );

    Ok(Command {
        java: java.to_path_buf(),
        args,
        working_dir: game_dir.to_path_buf(),
    })
}

#[cfg(test)]
#[path = "chemin.test.rs"]
mod tests;
