//! Tout ce qu'il faut réunir avant de lancer une partie.

use anyhow::Result;

use super::instance::ouvrir;
use mc_pack::lockfile::Lockfile;
use mc_pack::source::Source;

use mc_instance::launch::{Command, QuickPlay, Session};

/// Une partie prête à démarrer.
pub(super) struct Partie {
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

pub(super) async fn preparer(
    source: &Source,
    options: &mc_pack::Options,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
) -> Result<Partie> {
    let session = super::identite::choisir(pseudo).await?;
    let (manifest, lock, instance) = ouvrir(source, options)?;
    let layout = &options.layout;

    let version_id = mc_instance::neoforge::version_id(&lock.loader.version);

    // Le Java du verrou, pas celui du système : c'est avec lui que NeoForge a
    // été installé.
    let java = mc_java::ensure(lock.java, &layout.runtime()).await?;

    // À défaut de --serveur, celui que le pack déclare pour l'environnement de
    // ce binaire. Le manifeste est le même partout — c'est la même image de
    // contenu, servie sous trois noms — donc c'est au client de choisir, et il
    // choisit avec ce que la CI lui a figé à la compilation.
    let (cible, demande_explicite, environnement) = super::cible::choisir(&manifest, serveur);

    let launch_options = mc_instance::launch::LaunchOptions {
        memory_mb: memoire,
        quick_play: cible.clone().map(QuickPlay::Multiplayer),
        ..Default::default()
    };

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
