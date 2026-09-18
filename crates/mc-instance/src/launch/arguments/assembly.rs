//! The arguments in the order the JVM expects them.
//!
//! The base lays down its own, the loader adds its on top: the chain is
//! therefore walked in the reverse order of inheritance.

use std::collections::BTreeMap;
use std::path::Path;

use crate::vanilla::Features;

use super::super::descriptor::VersionJson;
use super::super::session::LaunchOptions;
use super::{collect, substitute};

#[allow(clippy::too_many_arguments)]
pub(in crate::launch) fn assemble(
    chain: &[VersionJson],
    os: &str,
    arch: &str,
    features: &Features,
    variables: &BTreeMap<String, String>,
    natives: &Path,
    classpath_text: &str,
    main_class: String,
    options: &LaunchOptions,
) -> Vec<String> {
    let mut jvm = Vec::new();
    let mut game = Vec::new();
    for version in chain.iter().rev() {
        collect(
            &version.arguments.jvm,
            os,
            arch,
            features,
            variables,
            &mut jvm,
        );
        collect(
            &version.arguments.game,
            os,
            arch,
            features,
            variables,
            &mut game,
        );
        if let Some(legacy) = &version.minecraft_arguments {
            game.extend(legacy.split_whitespace().map(|a| substitute(a, variables)));
        }
    }

    if jvm.is_empty() {
        // Pre-2017 format: the descriptor doesn't describe JVM arguments.
        jvm.push("-Djava.library.path=".to_string() + &natives.display().to_string());
        jvm.push("-cp".into());
        jvm.push(classpath_text.to_string());
    }

    let mut args = Vec::new();
    if let Some(mb) = options.memory_mb {
        // Before those from the descriptor, so that an explicit setting can
        // still be overridden by whatever the loader insists on.
        args.push(format!("-Xmx{mb}M"));
    }
    args.extend(options.extra_jvm.iter().cloned());
    args.extend(jvm);
    args.push(main_class);
    args.extend(game);

    // AFTER the descriptor's arguments, and that's the right spot: the
    // descriptor doesn't describe `--fullscreen`, and inserting it earlier
    // would shift the positional arguments the loader places.
    if options.fullscreen {
        args.push("--fullscreen".to_string());
    }

    args
}

#[cfg(test)]
#[path = "assembly.test.rs"]
mod tests;
