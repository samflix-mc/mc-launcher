//! Ce qu'on écrit au journal quand une commande se termine.

use std::process::ExitCode;
use std::time::Instant;

use anyhow::Result;

use mc_pack::source::Source;

/// Span racine : tout ce qui suit lui est rattaché, et sa durée est celle de la
/// commande. Dans le fichier comme dans Sentry, une exécution se lit ainsi d'un
/// bloc même quand plusieurs se succèdent.
pub fn ouvrir(command: &str, source: &Source) -> tracing::span::EnteredSpan {
    let span = tracing::info_span!(
        "commande",
        nom = %command,
        pack = %source.describe(),
        environnement = mc_log::environment::current().as_str(),
    );
    let entree = span.entered();
    tracing::info!(
        environnement = mc_log::environment::current().as_str(),
        "mc-pack {command} sur {} — environnement {}",
        source.describe(),
        mc_log::environment::current().as_str()
    );
    entree
}

/// Une erreur remontée jusqu'ici met fin au programme : c'est le dernier
/// endroit où elle peut devenir un incident plutôt qu'un simple message.
///
/// Hors de portée des tests de mutation : tout ce que fait cette fonction est
/// d'émettre une ligne de journal — que la couche Sentry transforme en
/// incident, ce qui se vérifie chez elle — et d'écrire le chemin du fichier
/// détaillé sur la sortie d'erreur. Rust ne permet de relire ni l'une ni
/// l'autre depuis le processus qui les produit.
#[mutants::skip]
pub fn conclure(command: &str, result: &Result<ExitCode>, debut: Instant, log: &mc_log::Guard) {
    match &result {
        Ok(_) => tracing::info!(
            duree_ms = debut.elapsed().as_millis(),
            "mc-pack {command} terminé en {:.1} s",
            debut.elapsed().as_secs_f64()
        ),
        Err(error) => {
            tracing::error!(
                duree_ms = debut.elapsed().as_millis(),
                erreur = ?error,
                "mc-pack {command} a échoué après {:.1} s : {error}",
                debut.elapsed().as_secs_f64()
            );
            if let Some(path) = log.log_path() {
                eprintln!("\nJournal détaillé : {}", path.display());
            }
        }
    }
}

#[cfg(test)]
#[path = "journal.test.rs"]
mod tests;
