//! The rules that decide whether a library or an argument applies.

use super::descriptor::{Features, Rule};

/// Does this Maven library's name hold for this system?
///
/// Rules are evaluated in order, the last one that applies wins. With no
/// rules at all, the library is kept — that's the case for two thirds of
/// them.
pub(crate) fn allowed(rules: &[Rule], os: &str, arch: &str) -> bool {
    allowed_with(rules, os, arch, &Features::new())
}

/// Same evaluation, taking active flags into account.
///
/// A rule conditioned on a flag only applies if the launcher has enabled it.
/// This is what keeps `--quickPlayMultiplayer` off the command line unless
/// joining a server was actually requested.
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
        // A flag expected to be `false` requires its absence: that's how
        // Mojang expresses "except in demo".
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
#[path = "rules.test.rs"]
mod tests;
