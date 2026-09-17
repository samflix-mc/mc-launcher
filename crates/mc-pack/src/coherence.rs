//! L'instance installée correspond-elle au verrou qu'on lit ?

use mc_mods::Side;

use crate::lockfile::Lockfile;

/// Mods que le verrou annonce côté client et que l'instance n'a pas.
///
/// Le verrou et l'instance ne viennent pas du même endroit. Le verrou est rangé
/// dans le cache du pack, qui est séparé par hôte : un pack de dev et un pack de
/// production ne se marchent pas dessus. L'instance, elle, est nommée d'après le
/// pack — « samflix » dans les trois cas, puisque c'est la même image de contenu
/// servie sous trois noms — et il n'y en a donc qu'une pour les trois.
///
/// Un `install` lancé avec un autre SAMFLIX_ENV remplace les jars de cette
/// instance unique sans que le verrou de l'autre environnement en sache rien.
/// Les deux se contredisent alors en silence, et c'est le serveur qui tranche,
/// par une éjection pour listes de mods divergentes — à cent lieues de sa cause.
///
/// Comparer les noms de fichiers suffit et ne coûte qu'un `stat` par mod. Les
/// empreintes sont l'affaire de `verify`, qui a le droit d'être lent.
pub fn mods_client_absents(lock: &Lockfile, instance: &mc_instance::Instance) -> Vec<String> {
    let mods_dir = instance.mods_dir();
    lock.mods
        .iter()
        .filter(|entry| {
            Side::parse(&entry.side)
                .unwrap_or(Side::Both)
                .includes(Side::Client)
        })
        .filter(|entry| !mods_dir.join(&entry.file_name).is_file())
        .map(|entry| entry.file_name.clone())
        .collect()
}

#[cfg(test)]
#[path = "coherence.test.rs"]
mod tests;
