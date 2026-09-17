//! Lancer, attendre, et rendre compte de ce qui s'est passé.

use anyhow::{Result, bail};

use super::super::incident::{report_game_crash, report_game_error};
use super::preparation::Partie;

pub(super) async fn jouer(partie: &Partie) -> Result<()> {
    let Partie {
        instance,
        lock,
        version_id,
        command,
        ..
    } = partie;

    // Daté avant le lancement : c'est ce qui permet d'écarter le rapport d'une
    // partie précédente, qui enverrait sur une fausse piste.
    let started_at = mc_instance::crash::now();

    let report = mc_instance::launch::run(command).await?;

    // Remontées avant le verdict : une erreur survenue pendant une partie qui
    // s'est bien terminée compte autant qu'un plantage, et c'est elle qu'on ne
    // verrait jamais autrement.
    for erreur in &report.errors {
        report_game_error(instance, lock, version_id, erreur, None);
    }
    if !report.errors.is_empty() {
        eprintln!(
            "\n{} erreurs relevées pendant la partie :",
            report.errors.len()
        );
        for erreur in &report.errors {
            eprintln!("  {} : {}", erreur.exception, erreur.message);
        }
    }

    match report.outcome {
        mc_instance::launch::Outcome::Normal => {
            tracing::info!("Minecraft s'est terminé normalement");
            Ok(())
        }
        // Fermer le jeu n'est pas une panne : le signaler comme telle
        // ouvrirait un incident à chaque partie terminée au clavier.
        mc_instance::launch::Outcome::Interrupted { signal } => {
            tracing::info!(signal, "Minecraft a été interrompu");
            println!("Minecraft a été interrompu (signal {signal}).");
            Ok(())
        }
        mc_instance::launch::Outcome::Failed { code } => {
            report_game_crash(instance, lock, version_id, started_at, code);
            bail!(
                "Minecraft s'est arrêté avec le code {code} — journaux dans {}",
                instance.game_dir.join("logs").display()
            )
        }
    }
}

#[cfg(test)]
#[path = "partie.test.rs"]
mod tests;
