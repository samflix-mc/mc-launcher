//! Assembly: from the chain of versions to the command line.

use anyhow::{Context, Result};
use std::path::Path;

use crate::vanilla;

use super::arguments::assemble;
use super::classpath::classpath;
use super::command::Command;
use super::descriptor::resolve_chain;
use super::session::{LaunchOptions, Session};
use super::variables::{active_features, variables};

#[tracing::instrument(
    name = "command line",
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
    let base = chain.last().expect("at least one version");

    let main_class = chain
        .iter()
        .find_map(|v| v.main_class.clone())
        .context("no main class in the version chain")?;
    let assets_index = chain
        .iter()
        .find_map(|v| v.asset_index.as_ref().map(|a| a.id.clone()))
        .or_else(|| chain.iter().find_map(|v| v.assets.clone()))
        .context("no assets index in the version chain")?;

    let os = vanilla::mojang_os();
    let arch = vanilla::mojang_arch();
    let features = active_features(options);
    let separator = if cfg!(windows) { ";" } else { ":" };

    let (classpath, classpath_text) = classpath(&chain, shared, os, arch, separator)?;

    // LWJGL drops the binaries it extracts from the jars there; it refuses
    // to start if the directory doesn't exist.
    let natives = shared.join("natives").join(&base.id);
    std::fs::create_dir_all(&natives).with_context(|| format!("creating {}", natives.display()))?;
    std::fs::create_dir_all(game_dir)
        .with_context(|| format!("creating {}", game_dir.display()))?;

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

    let args = assemble(
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
        libraries = classpath.len(),
        arguments = args.len(),
        "command line assembled"
    );

    Ok(Command {
        java: java.to_path_buf(),
        args,
        working_dir: game_dir.to_path_buf(),
    })
}

#[cfg(test)]
#[path = "path.test.rs"]
mod tests;
