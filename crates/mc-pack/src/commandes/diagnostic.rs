//! « Qu'est-ce qui ne va pas ? » — ce qu'on lance quand on ne sait pas encore.

use anyhow::Result;
use std::process::ExitCode;

/// Montre où vont les journaux et si la remontée d'incidents est active.
///
/// Première chose à demander à quelqu'un dont l'installation échoue : la
/// réponse tient en dix lignes et dit où trouver le reste.
pub fn diagnostic(log: &mc_log::Guard, incident_test: bool) -> Result<ExitCode> {
    println!("Journaux");
    match log.log_path() {
        Some(path) => println!("  fichier    : {}", path.display()),
        None => println!("  fichier    : indisponible (répertoire non inscriptible)"),
    }
    println!("  répertoire : {}", mc_log::log_dir().display());
    // Le même tri qu'à la pose du filtre : une RUST_LOG vide ne règle rien, et
    // l'afficher telle quelle rendait ici une ligne blanche — sur la commande
    // dont le seul travail est de dire ce qui est en vigueur.
    println!(
        "  console    : {}",
        std::env::var("RUST_LOG")
            .ok()
            .filter(|niveau| !niveau.trim().is_empty())
            .unwrap_or_else(|| "info (régler RUST_LOG)".into())
    );

    println!("\nRemontée d'incidents");
    println!(
        "  état       : {}",
        if mc_log::telemetry_active() {
            "active — couper avec SAMFLIX_TELEMETRY=0"
        } else {
            "coupée"
        }
    );
    println!(
        "  version    : {}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("inconnue")
    );
    // Affiché avec sa provenance : un environnement inattendu se remonte ainsi
    // à sa source sans avoir à relire le code.
    println!(
        "  environnement : {} ({})",
        mc_log::environment::current().as_str(),
        mc_log::environment::origin()
    );

    if incident_test {
        if !mc_log::telemetry_active() {
            println!("\nRien à envoyer : la remontée est coupée.");
            return Ok(ExitCode::SUCCESS);
        }
        println!("\nEnvoi d'un incident de test…");
        let (id, envoye) = mc_log::send_test_event();
        println!("  identifiant : {id}");
        if envoye {
            println!("  envoi       : abouti — à retrouver dans Sentry sous cet identifiant");
            println!("  canaux      : incident (Issues) et journal structuré (Logs)");
            println!("\n  Dans l'onglet Logs, sur « ligne de journal de test », un seul");
            println!("  attribut tranche — les deux autres sont filtrés par Sentry lui-même");
            println!("  et ne diraient rien de notre censure :");
            println!("    temoin_chemin → « ~/… »  : before_send_log a tourné");
            println!("                  → « /home/… » : il n'a pas tourné, il y a une fuite");
        } else {
            println!("  envoi       : ÉCHOUÉ (file non vidée avant expiration)");
            println!("                réseau bloqué, DSN erroné ou projet inexistant");
            return Ok(ExitCode::FAILURE);
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
