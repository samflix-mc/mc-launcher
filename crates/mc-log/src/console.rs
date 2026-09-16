//! La couche console : ce qu'un utilisateur voit pendant qu'il attend.

mod format;

use tracing_subscriber::{EnvFilter, Layer};

use crate::BoxedLayer;
use crate::fichier::Redacting;
use format::ConsoleFormat;

/// La console montre l'essentiel ; le fichier garde tout.
///
/// `RUST_LOG` règle la première sans toucher au second, pour qu'un utilisateur
/// qui augmente la verbosité n'ait pas à relancer l'opération qui a échoué.
pub(crate) fn layer() -> BoxedLayer {
    let filtre = filtre(std::env::var("RUST_LOG").ok().as_deref());

    tracing_subscriber::fmt::layer()
        .with_target(false)
        // Le temps écoulé depuis le démarrage, pas l'heure absolue. Une
        // commande dure quelques secondes : savoir qu'une étape a pris
        // 4,2 s renseigne, savoir qu'il était 01:18:38 non.
        // Les champs du span racine seraient répétés à chaque ligne —
        // « commande{nom=lock manifeste=… environnement=local} » sept fois
        // de suite noie ce qu'on cherche à lire. Le fichier les garde.
        .event_format(ConsoleFormat::new())
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(Redacting(std::io::stderr as fn() -> std::io::Stderr))
        .with_filter(filtre)
        .boxed()
}

/// Ce que `RUST_LOG` vaut, une fois écartés les deux cas qui rendraient la
/// console muette sans le dire.
fn filtre(brut: Option<&str>) -> EnvFilter {
    // Une RUST_LOG posée mais vide vaut une RUST_LOG absente : recopier le
    // « .env » d'exemple tel quel la pose ainsi, et `try_from_default_env`
    // rendrait alors un filtre sans la moindre directive — console muette,
    // défaut compris, sans que rien ne l'explique.
    brut.filter(|niveau| !niveau.trim().is_empty())
        .and_then(|niveau| match EnvFilter::try_new(niveau) {
            Ok(filtre) => Some(filtre),
            // Le souscripteur n'est pas encore posé : ce message ne peut passer
            // que par la sortie d'erreur. Le taire rendrait une RUST_LOG mal
            // écrite indiscernable d'une RUST_LOG absente — soit exactement le
            // silence inexpliqué que le cas précédent corrige.
            Err(erreur) => {
                eprintln!("RUST_LOG ignorée ({erreur}) : « {niveau} » — filtre par défaut.");
                None
            }
        })
        .unwrap_or_else(|| EnvFilter::new("info,hyper=warn,reqwest=warn,rustls=warn"))
}

#[cfg(test)]
#[path = "console.test.rs"]
mod tests;
