//! Les règles qui décident si une bibliothèque ou un argument s'applique.

use super::descripteur::{Features, Rule};

/// Le nom d'une bibliothèque Maven vaut-il pour ce système ?
///
/// Les règles sont évaluées dans l'ordre, la dernière qui s'applique l'emporte.
/// En l'absence de toute règle, la bibliothèque est retenue — c'est le cas des
/// deux tiers d'entre elles.
pub(crate) fn allowed(rules: &[Rule], os: &str, arch: &str) -> bool {
    allowed_with(rules, os, arch, &Features::new())
}

/// Même évaluation, en tenant compte des drapeaux actifs.
///
/// Une règle conditionnée à un drapeau ne s'applique que si le launcher l'a
/// activé. C'est ce qui fait que `--quickPlayMultiplayer` n'apparaît sur la
/// ligne de commande que lorsqu'on demande effectivement de rejoindre un
/// serveur.
pub(crate) fn allowed_with(rules: &[Rule], os: &str, arch: &str, features: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allow = false;
    for rule in rules {
        let os_ok = match &rule.os {
            None => true,
            Some(condition) => {
                condition.name.as_deref().map(|n| n == os).unwrap_or(true)
                    && condition.arch.as_deref().map(|a| a == arch).unwrap_or(true)
            }
        };
        // Un drapeau demandé à `false` exige son absence : c'est ainsi que
        // Mojang exprime « sauf en démo ».
        let features_ok = match &rule.features {
            None => true,
            Some(wanted) => wanted
                .iter()
                .all(|(name, expected)| features.contains(name) == *expected),
        };
        if os_ok && features_ok {
            allow = rule.action == "allow";
        }
    }
    allow
}

#[cfg(test)]
#[path = "regles.test.rs"]
mod tests;
