//! Vérifier qu'une installation est complète et intacte.

use anyhow::Result;

use crate::disposition::Layout;
use crate::{neoforge, vanilla};

/// Vérifie qu'une installation est complète et intacte.
///
/// `deep` recontrôle l'empreinte de chaque objet d'assets, ce que
/// l'installation ne fait pas pour ne pas relire 800 Mo à chaque lancement.
pub fn verify(
    mc: &str,
    neoforge_version: &str,
    layout: &Layout,
    deep: bool,
) -> Result<Vec<String>> {
    let shared = layout.shared();
    let mut problems = Vec::new();

    let version_json = shared.join("versions").join(mc).join(format!("{mc}.json"));
    let client_jar = shared.join("versions").join(mc).join(format!("{mc}.jar"));
    for path in [&version_json, &client_jar] {
        if !path.is_file() {
            problems.push(format!("fichier manquant : {}", path.display()));
        }
    }

    let neoforge_json = shared
        .join("versions")
        .join(neoforge::version_id(neoforge_version))
        .join(format!("{}.json", neoforge::version_id(neoforge_version)));
    if !neoforge_json.is_file() {
        problems.push(format!(
            "NeoForge {neoforge_version} n'est pas installé : {} absent",
            neoforge_json.display()
        ));
    }

    // Les deux descripteurs sont contrôlés : celui de NeoForge ajoute une
    // cinquantaine de bibliothèques au classpath, et il en manque une suffit à
    // faire échouer le démarrage aussi sûrement qu'une bibliothèque vanilla.
    for descriptor in [&version_json, &neoforge_json] {
        if !descriptor.is_file() {
            continue;
        }
        for library in vanilla::classpath(descriptor, &shared)? {
            if !library.is_file() {
                problems.push(format!("bibliothèque manquante : {}", library.display()));
            }
        }
    }

    if deep && problems.is_empty() {
        let index = shared.join("assets").join("indexes");
        let id = std::fs::read_dir(&index)
            .ok()
            .and_then(|entries| {
                entries.flatten().find_map(|e| {
                    e.path()
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
            })
            .unwrap_or_default();
        if !id.is_empty() {
            let report = vanilla::verify_assets(&shared, &id)?;
            for hash in report.missing {
                problems.push(format!("asset manquant : {hash}"));
            }
            for hash in report.corrupt {
                problems.push(format!("asset corrompu : {hash}"));
            }
        }
    }

    Ok(problems)
}
