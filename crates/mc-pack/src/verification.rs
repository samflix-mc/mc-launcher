//! Vérifier qu'une installation est complète et intacte.

use anyhow::{Context, Result};
use mc_mods::Side;

use crate::Options;
use crate::source::Source;

pub fn verify(source: &Source, options: &Options, deep: bool) -> Result<Vec<String>> {
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.with_context(|| {
        format!(
            "{} absent : rien à vérifier tant que « mc-pack install » n'a pas tourné",
            pack.lock_path.display()
        )
    })?;

    let mut problems = mc_instance::verify(
        &manifest.minecraft,
        &lock.loader.version,
        &options.layout,
        deep,
    )?;

    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let server_mods = instance.dir.join("server").join("mods");

    for entry in &lock.mods {
        let side = Side::parse(&entry.side).unwrap_or(Side::Both);
        let mut targets = Vec::new();
        if side.includes(Side::Client) {
            targets.push(instance.mods_dir().join(&entry.file_name));
        }
        if side.includes(Side::Server) {
            targets.push(server_mods.join(&entry.file_name));
        }

        for path in targets {
            if !path.is_file() {
                problems.push(format!("mod manquant : {}", path.display()));
                continue;
            }
            // Un verrou sans aucune empreinte ne permet pas de vérifier :
            // c'est le cas des entrées écrites depuis une source qui n'en
            // publiait pas, avant qu'on ne les calcule nous-mêmes.
            let Some(attendue) = entry.checksum() else {
                continue;
            };
            match std::fs::read(&path) {
                Ok(bytes) if attendue.matches(&bytes) => {}
                Ok(bytes) => problems.push(format!(
                    "{} : empreinte {} au lieu de {}",
                    path.display(),
                    attendue.of(&bytes),
                    attendue.expected()
                )),
                Err(e) => problems.push(format!("{} : illisible ({e})", path.display())),
            }
        }
    }

    // Le verrou porte les `modId` fournis par chaque jar : la cohérence de
    // l'ensemble se vérifie sans rouvrir une seule archive.
    let provided: std::collections::BTreeSet<&String> =
        lock.mods.iter().flat_map(|m| m.provides.iter()).collect();
    for missing in &lock.unresolved {
        if !provided.contains(&missing.mod_id) {
            problems.push(format!(
                "dépendance non satisfaite : {} exigé par {}",
                missing.mod_id, missing.required_by
            ));
        }
    }

    Ok(problems)
}
