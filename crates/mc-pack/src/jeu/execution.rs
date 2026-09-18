//! Lancer, attendre, et remonter ce qui s'est passé.
//!
//! Ce module ne décide pas de ce qu'on montre au joueur : il rend le rapport
//! brut, et l'appelant le met en forme — une ligne dans un terminal, un bandeau
//! dans une fenêtre. Ce qu'il fait, en revanche, aucun appelant ne doit avoir à
//! y penser : dater le lancement avant de lancer, et remonter les incidents.

use anyhow::Result;

use super::Partie;
use super::incidents::{report_game_crash, report_game_error};

/// Lance le jeu et attend qu'il se termine.
///
/// Le `Report` rendu porte le verdict et les erreurs relevées pendant la
/// partie. Un code de sortie non nul **n'est pas une erreur de cette
/// fonction** : le jeu a bien été lancé, et c'est à l'appelant de décider
/// comment l'annoncer. Remonter un `Err` ici obligerait la fenêtre à
/// reconstituer le code de sortie depuis un message.
///
/// Hors de portée des tests de mutation : elle lance Minecraft et attend qu'il
/// se termine. Ce qu'elle en tire est vérifié chez `mc_instance::launch`.
#[mutants::skip]
pub async fn jouer(partie: &Partie) -> Result<mc_instance::launch::Report> {
    jouer_annonce(partie, &crate::progression::Muet).await
}

/// Le même, en disant à qui écoute que le jeu a démarré.
///
/// L'annonce porte le numéro de processus : voir [`Rapport::partie_lancee`].
/// Sans elle, la fenêtre n'a aucun moyen de distinguer « on prépare » de « ça
/// tourne » — ce sont deux états du même appel, qui ne rend la main qu'à la
/// fin de la partie.
#[mutants::skip]
pub async fn jouer_annonce(
    partie: &Partie,
    rapport: &dyn crate::progression::Rapport,
) -> Result<mc_instance::launch::Report> {
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

    let report =
        mc_instance::launch::run_observe(command, &|pid| rapport.partie_lancee(pid)).await?;

    // Remontées avant le verdict : une erreur survenue pendant une partie qui
    // s'est bien terminée compte autant qu'un plantage, et c'est elle qu'on ne
    // verrait jamais autrement.
    for erreur in &report.errors {
        report_game_error(instance, lock, version_id, erreur, None);
    }

    match report.outcome {
        mc_instance::launch::Outcome::Normal => {
            tracing::info!("Minecraft s'est terminé normalement");
        }
        // Fermer le jeu n'est pas une panne : le signaler comme telle ouvrirait
        // un incident à chaque partie terminée au clavier.
        mc_instance::launch::Outcome::Interrupted { signal } => {
            tracing::info!(signal, "Minecraft a été interrompu");
        }
        mc_instance::launch::Outcome::Failed { code } => {
            tracing::warn!(code, "Minecraft s'est arrêté sur une erreur");
            report_game_crash(instance, lock, version_id, started_at, code);
        }
    }

    Ok(report)
}

/// Où le joueur trouvera les journaux du jeu, à citer quand ça a mal tourné.
pub fn journaux(partie: &Partie) -> std::path::PathBuf {
    partie.instance.game_dir.join("logs")
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
