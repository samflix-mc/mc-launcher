//! Installer le pack, puis dire ce qui a été posé.

use std::sync::Arc;

use anyhow::Result;
use mc_pack::source::Source;

use super::verify::report_unresolved;

/// Le rapport d'un terminal : des lignes, dans l'ordre où elles arrivent.
///
/// Les étapes ne sont pas imprimées — les notes les annoncent déjà, en
/// français et avec leurs chiffres. Elles servent à une fenêtre, qui doit
/// savoir *laquelle* travaille pour dessiner un chemin ; un terminal, lui,
/// empile.
///
/// Les téléchargements non plus : un débit qui change dix fois par seconde
/// défile plus vite qu'il ne se lit, et noierait le compte rendu.
struct Terminal;

impl mc_pack::Rapport for Terminal {
    fn etape(&self, _etape: mc_pack::Etape) {}

    /// Hors de portée des tests de mutation : elle n'écrit que sur la sortie
    /// standard, que Rust ne sait pas relire depuis le processus qui l'émet.
    /// Le TEXTE, lui, est calculé ailleurs — par `Suivi` et par les comptes
    /// rendus d'installation — et éprouvé là où il se calcule.
    #[mutants::skip]
    fn note(&self, texte: &str) {
        println!("{texte}");
    }
}

/// Hors de portée des tests de mutation : cette fonction installe le pack pour
/// de bon — elle télécharge Minecraft, NeoForge et cent mods — puis affiche le
/// résultat. Ce qu'elle affiche se vérifie ; ce qu'elle installe est vérifié
/// par la suite de `mc_pack::install`, qui a un serveur d'essai.
#[mutants::skip]
pub async fn install(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let outcome = mc_pack::install(source, options, Arc::new(Terminal)).await?;

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
