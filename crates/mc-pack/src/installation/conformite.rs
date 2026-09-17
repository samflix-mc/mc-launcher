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
//! ## Sur quoi porte la comparaison, et pourquoi pas sur le slug
//!
//! Sur le **projet**, identifié par sa source et son identifiant — et la valeur
//! comparée est le **build**, l'identifiant de version que le verrou épingle.
//!
//! Le slug ne convient pas, et c'est le piège dans lequel cette fonction est
//! tombée : le verrou nomme un mod CurseForge par son slug lisible
//! (`fix-gpu-memory-leak`), tandis que le rejeu le redemande par son
//! identifiant numérique (`882495`), que le candidat reprend tel quel. Comparés
//! par slug, ces deux-là sont deux mods différents — l'un manquant, l'autre en
//! trop — et chaque mod CurseForge du pack produisait deux écarts imaginaires.
//!
//! L'identifiant de projet, lui, est le même des deux côtés, quelle que soit la
//! façon dont on a demandé le mod.

use crate::lockfile::Lockfile;
use mc_mods::Origin;

/// Un mod réellement posé : d'où il vient, quel projet, quel build.
pub(super) type Pose<'a> = (Origin, &'a str, &'a str);

/// Les écarts entre le verrou rejoué et ce que la résolution a réellement
/// retenu. Vide quand les deux coïncident un pour un.
///
/// Prend des `Pose` plutôt qu'un `Plan` : la comparaison ne regarde que ces
/// trois champs, et un `Installed` n'est pas constructible hors de `mc-mods` —
/// la fonction serait invérifiable.
pub(super) fn ecarts<'a>(attendu: &Lockfile, poses: impl Iterator<Item = Pose<'a>>) -> Vec<String> {
    let poses: std::collections::BTreeMap<(Origin, &str), &str> = poses
        .map(|(origin, projet, build)| ((origin, projet), build))
        .collect();

    let mut ecarts = Vec::new();

    for exige in &attendu.mods {
        match poses.get(&(exige.source, exige.project.as_str())) {
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
    let exiges: std::collections::BTreeSet<(Origin, &str)> = attendu
        .mods
        .iter()
        .map(|m| (m.source, m.project.as_str()))
        .collect();
    for (origin, projet) in poses.keys() {
        if !exiges.contains(&(*origin, *projet)) {
            ecarts.push(format!(
                "{projet} ({}) : posé, absent du verrou",
                origin.as_str()
            ));
        }
    }

    ecarts
}

/// Une dépendance qu'un mod déclare vers un build précis.
pub(super) struct Exigence<'a> {
    /// Le mod qui exige, tel qu'on le nomme à l'écran.
    pub par: &'a str,
    pub origin: Origin,
    pub projet: &'a str,
    pub build: &'a str,
}

/// Les dépendances épinglées qu'un auteur déclare et que le pack ne respecte
/// pas.
///
/// Un mod peut exiger **un build précis** d'un autre, et non une plage : c'est
/// le cas d'Iris, dont les mixins de compatibilité visent une version exacte de
/// Sodium. Le launcher transforme bien cette exigence en demande épinglée —
/// mais une demande explicite du manifeste l'emporte sur une dépendance
/// déclarée, et c'est voulu : le manifeste est souverain.
///
/// Ce qui ne l'est pas, c'est le silence. Un manifeste qui demande « sodium »
/// sans version obtient le dernier build, Iris obtient ses mixins appliqués sur
/// une classe qui n'existe plus, et le jeu tombe à la première connexion avec
/// une `ClassNotFoundException` que personne ne relie au pack. Le dire ici
/// coûte une comparaison et fait gagner une soirée.
pub(super) fn dependances_insatisfaites<'a>(
    poses: &[Pose<'a>],
    exigences: impl Iterator<Item = Exigence<'a>>,
) -> Vec<String> {
    let poses: std::collections::BTreeMap<(Origin, &str), &str> = poses
        .iter()
        .map(|(origin, projet, build)| ((*origin, *projet), *build))
        .collect();

    exigences
        .filter_map(|exigence| {
            let pose = poses.get(&(exigence.origin, exigence.projet))?;
            // La dépendance est absente du pack : c'est le rôle de
            // `Plan::unresolved`, pas le nôtre. Ici on ne parle que des builds
            // présents mais différents de ce qui est exigé.
            (*pose != exigence.build).then(|| {
                format!(
                    "{} exige le build {} de {}, mais {pose} est installé",
                    exigence.par, exigence.build, exigence.projet
                )
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "conformite.test.rs"]
mod tests;
