//! Deux projets, un même `modId` : NeoForge n'en chargera qu'un.

use std::collections::BTreeMap;

use crate::Origin;
use crate::resolve::raison::Reason;

use super::plan::Installed;

/// demandé explicitement : le plus vérifiable et le plus intentionnel.
pub(super) fn deduplicate_by_mod_id(chosen: &mut BTreeMap<(Origin, String), Installed>) {
    let mut owner: BTreeMap<String, (Origin, String)> = BTreeMap::new();
    let mut drop_keys: Vec<(Origin, String)> = Vec::new();

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
                    let loser = if keep_previous {
                        key.clone()
                    } else {
                        let loser = previous.clone();
                        owner.insert(mod_id.clone(), key.clone());
                        loser
                    };
                    if !drop_keys.contains(&loser) {
                        drop_keys.push(loser);
                    }
                }
            }
        }
    }

    for key in drop_keys {
        chosen.remove(&key);
    }
}

#[cfg(test)]
#[path = "doublons.test.rs"]
mod tests;
