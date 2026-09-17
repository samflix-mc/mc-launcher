//! Installer un pack : six étapes, dans cet ordre et pas un autre.

mod chargeur;
mod compte_rendu;
mod java;
mod mods;
mod mojang;
mod verrou;

use anyhow::Result;

use crate::source::{Pack, Source};
use crate::{Options, Outcome, Progress};

#[tracing::instrument(
    name = "installation",
    skip(options, log),
    fields(pack, minecraft, source = %source.describe(), rejeu, serveur = options.with_server)
)]
pub async fn install(source: &Source, options: &Options, log: Progress<'_>) -> Result<Outcome> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    let Pack {
        manifest,
        lock: previous_lock,
        lock_path,
        replay,
        from_cache,
    } = source.load(&dl).await?;

    let replay = doit_rejouer(replay, options.locked);

    // Renseignés après lecture du manifeste : le span les porte, donc tout ce
    // qui suit est rattaché au pack sans avoir à le répéter à chaque ligne.
    tracing::Span::current().record("pack", &manifest.name);
    tracing::Span::current().record("minecraft", &manifest.minecraft);
    tracing::Span::current().record("rejeu", replay);

    if from_cache {
        log("Hors-ligne : pack repris de la dernière copie connue.");
    }

    let layout = &options.layout;
    let shared = layout.shared();

    let neoforge_version =
        chargeur::version(&manifest, previous_lock.as_ref(), &lock_path, replay, &dl).await?;
    log(&format!(
        "Minecraft {} — NeoForge {neoforge_version}",
        manifest.minecraft
    ));

    // --- 2. Fichiers de Mojang ----------------------------------------------
    let game = mojang::poser(&manifest.minecraft, &shared, &dl, log).await?;

    // --- 3. Java -------------------------------------------------------------
    let java_major = manifest.java_major(game.java_major);
    let java = java::runtime(java_major, layout).await?;
    log(&format!(
        "Java {} — {}",
        java.version.full,
        java.path.display()
    ));

    // --- 4. NeoForge ---------------------------------------------------------
    log("Chargeur NeoForge…");
    chargeur::poser(&neoforge_version, &shared, layout, &java.path, &dl).await?;

    // --- 5. Mods -------------------------------------------------------------
    let pose = mods::poser(
        &manifest,
        previous_lock.as_ref(),
        options,
        replay,
        &java.path,
        &neoforge_version,
        &dl,
        log,
    )
    .await?;

    let lock = verrou::retenir(
        &manifest,
        &neoforge_version,
        java_major,
        &pose.plan,
        previous_lock.as_ref(),
        &lock_path,
        replay,
    )?;

    Ok(compte_rendu::assembler(
        source,
        pose,
        java,
        game,
        lock,
        lock_path,
        previous_lock,
        neoforge_version,
        from_cache,
    ))
}

/// Faut-il rejouer le verrou plutôt que de résoudre à nouveau ?
///
/// Deux raisons, indépendantes l'une de l'autre. Un pack distant se rejoue
/// toujours : c'est le verrou publié qui décide des versions, pas la machine
/// du joueur. Et « --locked » l'exige explicitement, y compris sur un
/// manifeste local qu'on est en train d'éditer. Les confondre ferait résoudre
/// à nouveau un pack publié, et le joueur n'aurait pas les versions que le
/// réseau a validées.
fn doit_rejouer(pack_distant: bool, locked: bool) -> bool {
    pack_distant || locked
}

#[cfg(test)]
#[path = "installation.test.rs"]
mod tests;
