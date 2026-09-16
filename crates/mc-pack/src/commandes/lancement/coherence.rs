//! L'instance installée correspond-elle bien à ce verrou ?

use anyhow::{bail, Result};

use mc_pack::lockfile::Lockfile;

/// Le verrou vient du cache de ce pack-ci, qui est séparé par hôte ;
/// l'instance, elle, porte le nom du pack et il n'y en a qu'une pour les trois
/// environnements. Un « install » lancé avec un autre `SAMFLIX_ENV` a donc pu
/// remplacer ces jars sans que ce verrou en sache rien.
///
/// Démarrer quand même, c'est laisser le serveur trancher par une éjection
/// pour listes de mods divergentes — et cette éjection ne nomme pas sa cause.
pub(super) fn verifier(lock: &Lockfile, instance: &mc_instance::Instance) -> Result<()> {
    let manquants = mc_pack::mods_client_absents(lock, instance);
    if manquants.is_empty() {
        return Ok(());
    }
    bail!(
        "{} mods du verrou manquent dans {} : cette instance a été installée \
         depuis un autre pack que celui-ci.\n\
         Relancer « mc-pack install » — environnement « {} », {}.\n  {}",
        manquants.len(),
        instance.mods_dir().display(),
        mc_log::environment::current().as_str(),
        mc_log::environment::origin(),
        manquants.join("\n  ")
    )
}
