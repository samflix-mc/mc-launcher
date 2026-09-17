//! Deux projets, un même `modId` **racine** : NeoForge n'en chargera qu'un.
//!
//! « Racine » n'est pas un détail, c'est la correction d'un bug qui a coûté
//! trois mods au pack samflix. Un jar déclare son ou ses `modId` dans son
//! descripteur, et en apporte d'autres par les bibliothèques qu'il embarque.
//! Seuls les premiers disent qui il est ; les seconds, NeoForge sait les
//! dédupliquer lui-même au chargement.
//!
//! Les avoir confondus faisait passer Sodium et Iris pour un doublon — ils
//! partagent `fabric_api_base` et trois autres shims — et en supprimait un des
//! deux, sans message, avec un code de sortie nul.

use std::collections::BTreeMap;

use crate::Origin;
use crate::resolve::raison::Reason;

use super::plan::Installed;

/// Ce qu'un tour de déduplication a retiré, et pourquoi.
///
/// Rendu plutôt que tu : un mod qui disparaît du plan doit se voir. Le
/// résolveur rendait 0 en ayant perdu la moitié d'un manifeste, et seule la CI
/// de mc-content s'en apercevait, après coup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Eviction {
    /// Slug du projet retiré.
    pub ecarte: String,
    /// Slug du projet qui garde la place.
    pub retenu: String,
    /// Le `modId` racine que les deux déclarent.
    pub mod_id: String,
    /// Le projet retiré était-il demandé par le manifeste ?
    pub explicite: bool,
}

/// Écarte les projets qui déclarent un même `modId` **racine**.
///
/// Seuls les `modId` du descripteur comptent, jamais ceux des jars embarqués :
/// deux mods embarquent légitimement la même bibliothèque, et NeoForge sait les
/// dédupliquer au chargement. Les avoir confondus faisait passer Sodium et Iris
/// pour un doublon — ils partagent quatre shims Fabric — et supprimait l'un des
/// deux en silence, alors qu'un pack qui garde Iris sans Sodium est cassé.
///
/// Entre deux prétendants, on garde le plus vérifiable puis le plus
/// intentionnel : celui qui porte une empreinte, puis celui qui a été demandé
/// explicitement.
pub(super) fn deduplicate_by_mod_id(
    chosen: &mut BTreeMap<(Origin, String), Installed>,
) -> Vec<Eviction> {
    let mut owner: BTreeMap<String, (Origin, String)> = BTreeMap::new();
    let mut drop_keys: Vec<(Origin, String)> = Vec::new();
    let mut evictions: Vec<Eviction> = Vec::new();

    for (key, entry) in chosen.iter() {
        for mod_id in &entry.provides {
            match owner.get(mod_id) {
                None => {
                    owner.insert(mod_id.clone(), key.clone());
                }
                Some(previous) => {
                    let keep_previous = {
                        let other = &chosen[previous];
                        let score = |m: &Installed| {
                            (
                                m.candidate.checksum().is_some(),
                                m.reason == Reason::Explicit,
                            )
                        };
                        score(other) >= score(entry)
                    };
                    let (loser, winner) = if keep_previous {
                        (key.clone(), previous.clone())
                    } else {
                        let loser = previous.clone();
                        owner.insert(mod_id.clone(), key.clone());
                        (loser, key.clone())
                    };
                    if !drop_keys.contains(&loser) {
                        let ecarte = &chosen[&loser];
                        evictions.push(Eviction {
                            ecarte: ecarte.candidate.slug.clone(),
                            retenu: chosen[&winner].candidate.slug.clone(),
                            mod_id: mod_id.clone(),
                            explicite: ecarte.reason == Reason::Explicit,
                        });
                        drop_keys.push(loser);
                    }
                }
            }
        }
    }

    for key in drop_keys {
        chosen.remove(&key);
    }
    evictions
}

#[cfg(test)]
#[path = "doublons.test.rs"]
mod tests;
