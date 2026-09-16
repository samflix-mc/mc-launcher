//! Les arguments que le descripteur décrit, et ceux qu'il faut inventer.

mod assemblage;

use std::collections::BTreeMap;

use crate::vanilla::{self, Features};

use super::descripteur::Argument;

pub(super) use assemblage::assembler;

fn collect(
    arguments: &[Argument],
    os: &str,
    arch: &str,
    features: &Features,
    variables: &BTreeMap<String, String>,
    out: &mut Vec<String>,
) {
    for argument in arguments {
        match argument {
            Argument::Simple(value) => out.push(substitute(value, variables)),
            Argument::Conditional { rules, value } => {
                if !vanilla::allowed_with(rules, os, arch, features) {
                    continue;
                }
                for part in value.parts() {
                    out.push(substitute(part, variables));
                }
            }
        }
    }
}

/// Remplace les `${…}` par leur valeur.
///
/// Une variable inconnue est laissée telle quelle plutôt que vidée : un
/// argument qui garde `${quelque_chose}` se remarque dans un message d'erreur,
/// là où un argument devenu vide décale silencieusement tous les suivants.
pub(super) fn substitute(text: &str, variables: &BTreeMap<String, String>) -> String {
    if !text.contains("${") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match variables.get(name) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push_str("${");
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[path = "arguments.test.rs"]
mod tests;
