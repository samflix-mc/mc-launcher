//! Les arguments dans l'ordre où la JVM les attend.
//!
//! Le socle pose les siens, le chargeur ajoute les siens par-dessus : la
//! chaîne est donc parcourue à l'envers de l'héritage.

use std::collections::BTreeMap;
use std::path::Path;

use crate::vanilla::Features;

use super::super::descripteur::VersionJson;
use super::super::session::LaunchOptions;
use super::{collect, substitute};

#[allow(clippy::too_many_arguments)]
pub(in crate::launch) fn assembler(
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
        collect(&version.arguments.jvm, os, arch, features, variables, &mut jvm);
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
        // Format d'avant 2017 : le descripteur ne décrit pas les arguments JVM.
        jvm.push("-Djava.library.path=".to_string() + &natives.display().to_string());
        jvm.push("-cp".into());
        jvm.push(classpath_text.to_string());
    }

    let mut args = Vec::new();
    if let Some(mb) = options.memory_mb {
        // Avant ceux du descripteur, pour qu'un réglage explicite puisse être
        // contredit par ce que le chargeur impose s'il y tient.
        args.push(format!("-Xmx{mb}M"));
    }
    args.extend(options.extra_jvm.iter().cloned());
    args.extend(jvm);
    args.push(main_class);
    args.extend(game);
    args
}
