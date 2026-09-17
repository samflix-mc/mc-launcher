//! Ce que la commande affiche du verrou qu'elle vient d'écrire.

use std::path::Path;

use mc_pack::lockfile::Lockfile;

use super::super::verify::report_unresolved;

/// Hors de portée des tests de mutation : cette fonction n'a d'autre effet que
/// d'écrire sur la sortie standard. Ce qu'elle affiche se vérifie — c'est
/// [`lignes`], juste en dessous.
#[mutants::skip]
pub(super) fn annoncer(lock: &Lockfile, lock_path: &Path, previous: Option<&Lockfile>) {
    for ligne in lignes(lock, lock_path, previous) {
        println!("{ligne}");
    }
    report_unresolved(lock);
}

/// Le rapport, séparé de son affichage.
///
/// C'est ce qu'on relit pour savoir ce qu'une résolution a changé : le
/// chargeur, le nombre de mods, chacun avec son côté et qui l'a réclamé, puis
/// les différences avec le verrou précédent. Taire ces dernières laisserait
/// croire qu'une résolution n'a rien changé alors qu'elle a remplacé dix
/// builds.
pub(super) fn lignes(
    lock: &Lockfile,
    lock_path: &Path,
    previous: Option<&Lockfile>,
) -> Vec<String> {
    let mut lignes = vec![format!(
        "NeoForge {} — {} mods",
        lock.loader.version,
        lock.mods.len()
    )];

    lignes.extend(lock.mods.iter().map(|entry| {
        format!(
            "  {:<24} {:<40} {:<7} {}",
            entry.slug, entry.file_name, entry.side, entry.reason
        )
    }));

    if let Some(previous) = previous {
        let changes = lock.diff(previous);
        if !changes.is_empty() {
            lignes.push("\nChangements :".to_string());
            lignes.extend(changes.into_iter().map(|line| format!("  {line}")));
        }
    }

    lignes.push(format!("\nVerrou écrit dans {}", lock_path.display()));
    lignes
}

#[cfg(test)]
#[path = "rapport.test.rs"]
mod tests;
