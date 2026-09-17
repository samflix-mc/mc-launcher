//! Quel serveur rejoindre, et d'où vient ce choix.

use mc_pack::manifest::Manifest;

/// À défaut de `--serveur`, celui que le pack déclare pour l'environnement de
/// ce binaire.
///
/// Le manifeste est le même partout — c'est la même image de contenu, servie
/// sous trois noms — donc c'est au client de choisir, et il choisit avec ce que
/// la CI lui a figé à la compilation.
///
/// Une absence n'est pas une erreur : la préproduction n'a pas de serveurs
/// Minecraft derrière elle, et le jeu s'y lance sans rejoindre quoi que ce soit.
pub(super) fn choisir(
    manifest: &Manifest,
    serveur: Option<String>,
) -> (Option<String>, bool, mc_log::Environment) {
    let environnement = mc_log::environment::current();
    let demande_explicite = serveur.is_some();
    let cible = serveur.or_else(|| {
        manifest
            .server_for(environnement)
            .map(mc_pack::manifest::Server::address)
    });
    (cible, demande_explicite, environnement)
}

#[cfg(test)]
#[path = "cible.test.rs"]
mod tests;
