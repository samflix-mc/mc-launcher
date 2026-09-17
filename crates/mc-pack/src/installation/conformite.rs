//! Ce qui a été posé correspond-il à ce que le verrou annonçait ?
//!
//! Rejouer un verrou publié, c'est lui obéir. Encore faut-il vérifier qu'on y
//! est arrivé : entre le verrou et l'instance, il y a une résolution, des
//! téléchargements, des sources qui peuvent avoir retiré un build, et une
//! déduplication par `modId` qui peut écarter un jar au profit d'un autre.
//!
//! Rien de tout cela ne lève d'erreur — l'installation se termine « bien » —
//! et l'écart n'apparaît qu'au démarrage du jeu, sous la forme d'un NeoForge
//! qui refuse de charger, ou pire, d'une éjection du serveur pour listes de
//! mods divergentes. Une éjection ne nomme pas sa cause.
//!
//! La comparaison se fait donc ici, tout de suite, et sur ce qui compte : le
//! **build** de chaque mod, c'est-à-dire l'identifiant de version que le verrou
//! épingle. Comparer les slugs ne verrait pas qu'une version a glissé ;
//! comparer les empreintes verrait des différences que la source a le droit
//! d'avoir.

use crate::lockfile::Lockfile;

/// Les écarts entre le verrou rejoué et ce que la résolution a réellement
/// retenu. Vide quand les deux coïncident un pour un.
///
/// Prend des couples `(slug, build)` plutôt qu'un `Plan` : la comparaison ne
/// regarde que ces deux champs, et un `Installed` n'est pas constructible hors
/// de `mc-mods` — la fonction serait invérifiable.
pub(super) fn ecarts<'a>(
    attendu: &Lockfile,
    poses: impl Iterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let poses: std::collections::BTreeMap<&str, &str> = poses.collect();

    let mut ecarts = Vec::new();

    for exige in &attendu.mods {
        match poses.get(exige.slug.as_str()) {
            None => ecarts.push(format!("{} : épinglé par le verrou, absent", exige.slug)),
            Some(pose) if *pose != exige.file => ecarts.push(format!(
                "{} : build {} attendu, {pose} posé",
                exige.slug, exige.file
            )),
            Some(_) => {}
        }
    }

    // L'inverse compte autant : un mod que le verrou n'annonce pas est un mod
    // que les serveurs n'ont pas. NeoForge négocie ses registres à la
    // connexion, et un jar en trop fait échouer la négociation aussi sûrement
    // qu'un jar en moins.
    let exiges: std::collections::BTreeSet<&str> =
        attendu.mods.iter().map(|m| m.slug.as_str()).collect();
    for slug in poses.keys() {
        if !exiges.contains(slug) {
            ecarts.push(format!("{slug} : posé, absent du verrou"));
        }
    }

    ecarts
}

#[cfg(test)]
#[path = "conformite.test.rs"]
mod tests;
