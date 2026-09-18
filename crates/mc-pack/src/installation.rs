//! Installer un pack : six étapes, dans cet ordre et pas un autre.

mod chargeur;
mod compte_rendu;
mod conformite;
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

    // --- 1 bis. La purge, s'il y a lieu ---------------------------------------
    //
    // Ici et pas ailleurs : le manifeste est lu — donc on connaît la génération
    // demandée — et rien n'a encore été écrit dans l'instance.
    //
    // L'instance n'est PAS en portée à ce point : elle n'est construite qu'à
    // l'étape 6. On la dérive donc avec EXACTEMENT la même expression, sous
    // peine de purger un autre répertoire que celui qu'on remplira. Le nom est
    // sûr : `manifest/controle.rs:37-44` refuse déjà un `name` qui sortirait de
    // la racine, et c'est ce contrôle-là qui autorise un effacement récursif
    // sur un chemin venu du réseau.
    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let etat_pose = crate::etat::EtatLocal::lire(&crate::etat::chemin(&instance));

    // La génération vient du MANIFESTE et jamais du verrou. Au point où l'on
    // est, `previous_lock` est le verrou *publié* en rejeu, mais le verrou
    // *précédent* sur un manifeste local qu'on rerésout : les deux ne
    // répondent pas à la même question.
    let purge = match crate::etat::decider(etat_pose.as_ref(), manifest.generation) {
        crate::etat::Avant::Differentiel => crate::etat::Purge::default(),
        crate::etat::Avant::Purger => {
            // La note dit « en cours », le compte rendu dira ce qui a été fait :
            // `note` est un emplacement unique que sept appels ultérieurs vont
            // écraser dans la seconde.
            rapport.note("Réinstallation complète demandée par le pack…");
            tracing::info!(
                generation_posee = etat_pose.as_ref().map(|e| e.generation),
                generation_demandee = manifest.generation,
                "purge avant installation"
            );
            crate::etat::purger(&instance.game_dir)
        }
    };

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
    let java_major = manifest.java_major(Some(game.java_major));
    let java = java::runtime(java_major, layout, &rapport).await?;
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

    // Rejouer un verrou publié, c'est lui obéir — encore faut-il vérifier
    // qu'on y est arrivé. Un build retiré de sa source, une déduplication qui
    // a tranché autrement : l'installation se termine « bien », et l'écart
    // n'apparaît qu'à la connexion, sous la forme d'une éjection qui ne nomme
    // pas sa cause.
    let poses: Vec<conformite::Pose<'_>> = pose
        .plan
        .mods
        .iter()
        .map(|m| {
            (
                m.candidate.origin,
                m.candidate.project_id.as_str(),
                m.candidate.version_id.as_str(),
            )
        })
        .collect();

    let mut ecarts = Vec::new();

    if replay {
        ecarts.extend(conformite::ecarts(&lock, poses.iter().copied()));
    }

    // Un auteur peut exiger un build précis d'un autre mod — les mixins de
    // compatibilité d'Iris visent une version exacte de Sodium. Une demande
    // explicite du manifeste l'emporte sur cette exigence, et c'est voulu ;
    // n'en rien dire ne l'est pas, parce que le jeu tombe alors à la première
    // connexion sur une erreur qui ne nomme jamais le pack.
    ecarts.extend(conformite::dependances_insatisfaites(
        &poses,
        pose.plan.mods.iter().flat_map(|m| {
            m.candidate.declared_deps.iter().filter_map(move |dep| {
                Some(conformite::Exigence {
                    par: m.candidate.slug.as_str(),
                    origin: m.candidate.origin,
                    projet: dep.project_id.as_str(),
                    build: dep.version_id.as_deref()?,
                })
            })
        }),
    ));

    for ecart in &ecarts {
        tracing::warn!(ecart, "l'installation s'écarte de ce qui est attendu");
        rapport.note(&format!("  ⚠ {ecart}"));
    }

    // --- 8. Retenir ce qu'on vient de poser ----------------------------------
    //
    // Après tout le reste : un état écrit avant la fin décrirait une
    // installation qui n'a pas abouti, et le prochain lancement croirait n'avoir
    // rien à faire. Un échec d'écriture ne fait pas échouer l'installation —
    // elle a réussi — mais il coûte une purge au prochain lancement, et c'est
    // assez ennuyeux pour être journalisé.
    match lock.empreinte() {
        Ok(empreinte) => {
            let etat = crate::etat::EtatLocal::neuf(
                empreinte,
                manifest.generation,
                crate::lockfile::now_utc(),
            );
            if let Err(erreur) = etat.ecrire(&crate::etat::chemin(&instance)) {
                tracing::warn!(
                    erreur = %erreur,
                    "état local non écrit : le prochain lancement repassera par une purge"
                );
            }
        }
        Err(erreur) => tracing::warn!(erreur = %erreur, "empreinte du verrou incalculable"),
    }

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
        ecarts,
        purge,
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
