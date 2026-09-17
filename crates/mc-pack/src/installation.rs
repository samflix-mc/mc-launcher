//! Installer un pack : six étapes, dans cet ordre et pas un autre.

mod chargeur;
mod compte_rendu;
mod java;
mod mods;
mod mojang;
mod verrou;

use std::sync::Arc;

use anyhow::Result;

use crate::progression::{Etape, Rapport};
use crate::source::{Pack, Source};
use crate::{Options, Outcome};

#[tracing::instrument(
    name = "installation",
    skip(options, rapport),
    fields(pack, minecraft, source = %source.describe(), rejeu, serveur = options.with_server)
)]
pub async fn install(
    source: &Source,
    options: &Options,
    rapport: Arc<dyn Rapport>,
) -> Result<Outcome> {
    // Le client HTTP porte l'observateur : tout ce qui descend ensuite — le
    // pack, les fichiers du jeu, l'installateur NeoForge — passe par lui et
    // se raconte sans que chaque étape ait à s'en occuper.
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?.observe(observateur(&rapport));

    // --- 1. Le pack ----------------------------------------------------------
    rapport.etape(Etape::Pack);
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
        rapport.note("Hors-ligne : pack repris de la dernière copie connue.");
    }

    let layout = &options.layout;
    let shared = layout.shared();

    // --- 2. Le chargeur ------------------------------------------------------
    rapport.etape(Etape::Chargeur);
    let neoforge_version =
        chargeur::version(&manifest, previous_lock.as_ref(), &lock_path, replay, &dl).await?;
    rapport.note(&format!(
        "Minecraft {} — NeoForge {neoforge_version}",
        manifest.minecraft
    ));

    // --- 3. Fichiers de Mojang ----------------------------------------------
    rapport.etape(Etape::Minecraft);
    let game = mojang::poser(&manifest.minecraft, &shared, &dl, rapport.as_ref()).await?;

    // --- 4. Java -------------------------------------------------------------
    rapport.etape(Etape::Java);
    let java_major = manifest.java_major(game.java_major);
    let java = java::runtime(java_major, layout).await?;
    rapport.note(&format!(
        "Java {} — {}",
        java.version.full,
        java.path.display()
    ));

    // --- 5. NeoForge ---------------------------------------------------------
    rapport.etape(Etape::NeoForge);
    rapport.note("Chargeur NeoForge…");
    chargeur::poser(&neoforge_version, &shared, layout, &java.path, &dl).await?;

    // --- 6. Mods -------------------------------------------------------------
    rapport.etape(Etape::Mods);
    let pose = mods::poser(
        &manifest,
        previous_lock.as_ref(),
        options,
        replay,
        &java.path,
        &neoforge_version,
        &dl,
        &rapport,
    )
    .await?;

    // --- 7. Le verrou --------------------------------------------------------
    rapport.etape(Etape::Verrou);
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

/// Le fil qui relie les téléchargements au rapport.
///
/// `mc-dl` ne connaît que des closures, `mc-pack` ne connaît qu'un rapport :
/// ceci est la soudure, et le seul endroit du crate où les deux se voient.
pub(crate) fn observateur(rapport: &Arc<dyn Rapport>) -> mc_dl::Observateur {
    let rapport = Arc::clone(rapport);
    Arc::new(move |avancement| rapport.telechargement(avancement))
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
