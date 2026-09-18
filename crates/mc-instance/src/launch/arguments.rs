//! The arguments the descriptor describes, and the ones that must be invented.

mod assembly;

use std::collections::BTreeMap;

use crate::vanilla::{self, Features};

use super::descriptor::Argument;

pub(super) use assembly::assemble;

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

/// Replaces `${…}` with their value.
///
/// An unknown variable is left as-is rather than cleared: an argument that
/// keeps `${something}` stands out in an error message, whereas an argument
/// that becomes empty silently shifts every argument after it.
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
