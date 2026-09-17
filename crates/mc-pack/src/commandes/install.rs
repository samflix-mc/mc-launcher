//! Installer le pack, puis dire ce qui a été posé.

use anyhow::Result;
use mc_pack::source::Source;

use super::verify::report_unresolved;

/// Hors de portée des tests de mutation : cette fonction installe le pack pour
/// de bon — elle télécharge Minecraft, NeoForge et cent mods — puis affiche le
/// résultat. Ce qu'elle affiche se vérifie ; ce qu'elle installe est vérifié
/// par la suite de `mc_pack::install`, qui a un serveur d'essai.
#[mutants::skip]
pub async fn install(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let outcome = mc_pack::install(source, options, &|line| println!("{line}")).await?;

    for ligne in lignes(&outcome) {
        println!("{ligne}");
    }
    report_unresolved(&outcome.lock);
    Ok(())
}

/// Le compte rendu d'une installation, séparé de son affichage.
///
/// C'est ce qu'un joueur relit pour savoir où le jeu a été posé, et ce qu'une
/// mise à jour a changé. Deux listes n'y paraissent que si elles ont quelque
/// chose à dire : les mods retirés, et les différences avec le verrou
/// précédent. Un titre suivi du vide ferait chercher un changement qui n'a pas
/// eu lieu.
pub(super) fn lignes(outcome: &mc_pack::Outcome) -> Vec<String> {
    let mut lignes = vec![
        format!("\nInstance « {} »", outcome.instance.name),
        format!("  pack     : {}", outcome.source),
    ];
    if outcome.from_cache {
        lignes.push("             (hors-ligne — copie locale, pas le pack publié)".to_string());
    }
    lignes.push(format!(
        "  jeu      : {}",
        outcome.instance.game_dir.display()
    ));
    lignes.push(format!(
        "  mods     : {} côté client, {} côté serveur",
        outcome.client_mods, outcome.server_mods
    ));
    lignes.push(format!("  serveur  : {}", outcome.server_dir.display()));
    lignes.push(format!("  verrou   : {}", outcome.lock_path.display()));

    if !outcome.removed.is_empty() {
        lignes.push(format!("  retirés  : {}", outcome.removed.join(", ")));
    }
    if let Some(previous) = &outcome.previous_lock {
        let changes = outcome.lock.diff(previous);
        if !changes.is_empty() {
            lignes.push("\nChangements depuis le verrou précédent :".to_string());
            lignes.extend(changes.into_iter().map(|line| format!("  {line}")));
        }
    }
    lignes
}

#[cfg(test)]
#[path = "install.test.rs"]
mod tests;
