//! Mise en place des trois destinations.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::guard::{Guard, current_log_name};
use crate::redact::redact;
use crate::{BoxedLayer, console, fichier, incidents};

/// Met en place la journalisation pour un composant donné.
///
/// `component` nomme le binaire — il préfixe le fichier de journal et étiquette
/// les incidents, ce qui permet de distinguer un échec d'installation d'un
/// échec d'authentification sans ouvrir le rapport.
pub fn init(component: &str) -> Guard {
    // Avant Sentry : celui-ci chaîne son gestionnaire par-dessus l'existant.
    // Posé après, le nôtre le remplacerait et plus aucune panique ne serait
    // rapportée.
    install_panic_hook();
    let sentry_guard = incidents::init_sentry(component);
    let (file_layer, file_guard, log_dir) = fichier::file_layer(component);

    let mut layers: Vec<BoxedLayer> = Vec::new();
    layers.push(console::layer());
    if let Some(layer) = file_layer {
        layers.push(layer);
    }
    if sentry_guard.is_some() {
        layers.push(incidents::couche());
    }

    tracing_subscriber::registry().with(layers).init();

    let journal = log_dir.map(|dir| (dir, component.to_string()));
    if let Some((dir, component)) = &journal {
        tracing::debug!(
            fichier = %dir.join(current_log_name(component)).display(),
            "journal ouvert"
        );
    }

    Guard::new(file_guard, sentry_guard, journal)
}

/// Remplace l'affichage par défaut d'une panique.
///
/// Le gestionnaire standard de Rust écrit le message de panique directement sur
/// la sortie d'erreur, sans passer par `tracing` : il échappe donc à la censure
/// et au fichier de journal. Deux conséquences, toutes deux constatées avant
/// d'écrire ceci — un jeton présent dans un message de panique s'affichait en
/// clair dans le terminal, et la panique restait absente du fichier qu'on
/// demande justement de joindre.
///
/// Le message part en `warn` et non en `error` : l'incident lui-même est
/// rapporté par le gestionnaire de Sentry, avec sa trace d'appels complète, et
/// un `error` ici le ferait remonter une seconde fois.
fn install_panic_hook() {
    // Le gestionnaire d'origine n'est délibérément pas rappelé : il réécrirait
    // le message non censuré sur la sortie d'erreur, ce qui est précisément ce
    // qu'on vient d'éviter.
    std::panic::set_hook(Box::new(move |info| {
        let message = redact(&info.to_string());
        eprintln!("\n{message}");
        eprintln!("(relancer avec RUST_BACKTRACE=1 pour la trace d'appels)");
        tracing::warn!(panique = %message, "le programme s'est arrêté sur une panique");
    }));
}

#[cfg(test)]
#[path = "init.test.rs"]
mod tests;
