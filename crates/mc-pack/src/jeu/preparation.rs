//! Tout ce qu'il faut réunir avant de lancer une partie.

use anyhow::Result;

use super::instance::ouvrir;
use crate::lockfile::Lockfile;
use crate::source::Source;

use mc_instance::launch::{Command, QuickPlay, Session};

/// Une partie prête à démarrer.
pub struct Partie {
    pub instance: mc_instance::Instance,
    pub lock: Lockfile,
    pub version_id: String,
    pub command: Command,
    pub session: Session,
    /// Le serveur à rejoindre, et s'il vient de la ligne de commande.
    pub cible: Option<String>,
    pub demande_explicite: bool,
    pub environnement: mc_log::Environment,
}

pub async fn preparer(
    source: &Source,
    options: &crate::Options,
    identite: super::Identite,
    serveur: Option<String>,
    memoire: Option<u32>,
    rapport: std::sync::Arc<dyn crate::progression::Rapport>,
) -> Result<Partie> {
    let session = super::identite::choisir(identite).await?;
    let (manifest, lock, instance) = ouvrir(source, options)?;
    let layout = &options.layout;

    let version_id = mc_instance::neoforge::version_id(&lock.loader.version);

    let java = java_du_verrou(&lock, layout, &rapport).await?;

    // À défaut de --serveur, celui que le pack déclare pour l'environnement de
    // ce binaire. Le manifeste est le même partout — c'est la même image de
    // contenu, servie sous trois noms — donc c'est au client de choisir, et il
    // choisit avec ce que la CI lui a figé à la compilation.
    let (cible, demande_explicite, environnement) = super::cible::choisir(&manifest, serveur);

    let launch_options = options_de_lancement(memoire, cible.clone());

    let command = mc_instance::launch::build(
        &version_id,
        &layout.shared(),
        &instance.game_dir,
        &java.path,
        &session,
        &launch_options,
    )?;

    Ok(Partie {
        instance,
        lock,
        version_id,
        command,
        session,
        cible,
        demande_explicite,
        environnement,
    })
}

/// Le Java du verrou, pas celui du système : c'est avec lui que NeoForge a été
/// installé.
///
/// ## Pourquoi `detect` puis `install`, et non `ensure`
///
/// `ensure` ferait la même chose, mais sans dire lequel des deux cas s'est
/// produit — et c'est justement ce qu'il faut savoir pour ne pas mentir à
/// l'écran.
///
/// Au moment où l'on prépare une partie, la cinématique est déjà sur
/// `Phase::Pret`. `Suivi::phase` écrase sans garde de monotonie : émettre
/// l'étape « Java » inconditionnellement ferait RECULER l'affichage de la
/// dernière étape à la quatrième, à chaque lancement, sur un pack pourtant
/// complet. Le joueur verrait son launcher revenir en arrière sans raison.
///
/// On ne l'émet donc que dans le cas où quelque chose va réellement se passer
/// — un runtime à poser, cent quatre-vingts mégaoctets à descendre — et c'est
/// alors une reprise annoncée, qui a un sens.
async fn java_du_verrou(
    lock: &Lockfile,
    layout: &mc_instance::Layout,
    rapport: &std::sync::Arc<dyn crate::progression::Rapport>,
) -> Result<mc_java::Java> {
    if let Some(java) = mc_java::detect(lock.java, &layout.runtime()).await {
        tracing::debug!(
            version = %java.version.full,
            "Java du verrou déjà présent, rien à annoncer"
        );
        return Ok(java);
    }

    rapport.etape(crate::progression::Etape::Java);
    rapport.note(&format!(
        "Java {} manquant : installation avant de lancer…",
        lock.java
    ));
    mc_java::install(
        lock.java,
        &layout.runtime(),
        Some(crate::installation::observateur(rapport)),
    )
    .await
}

/// Ce que la ligne de commande demande au jeu lui-même.
///
/// Deux réglages, et deux façons de les perdre en silence. Sans `memory_mb`,
/// la JVM retombe sur son défaut — un quart de la mémoire de la machine, ce
/// qui ne suffit pas à un modpack et donne un `OutOfMemoryError` au bout de
/// vingt minutes. Sans `quick_play`, le jeu s'ouvre sur son menu au lieu de
/// rejoindre le serveur, et l'on croit que le pack n'en déclare pas.
fn options_de_lancement(
    memoire: Option<u32>,
    cible: Option<String>,
) -> mc_instance::launch::LaunchOptions {
    mc_instance::launch::LaunchOptions {
        memory_mb: memoire,
        quick_play: cible.map(QuickPlay::Multiplayer),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "preparation.test.rs"]
mod tests;
