//! Sentry : les incidents seuls, paniques et erreurs.
//!
//! Un rapport automatique évite d'avoir à demander à un joueur de reproduire un
//! bug qu'il a déjà rencontré. Tout ce qui sort par là est censuré deux fois :
//! par la couche qui émet, et par les filtres du client.

mod client;
mod couche;
mod essai;
mod jeu;
mod scrub;

pub(crate) use client::init_sentry;
pub(crate) use couche::couche;
pub use essai::send_test_event;
pub use jeu::capture_game_crash;

/// Projet Sentry du launcher.
///
/// Un DSN n'est pas un secret : il ne permet que d'écrire des événements, et
/// tout client de bureau embarque le sien. Il reste remplaçable par
/// `SENTRY_DSN`, ce qui permet de router les incidents d'un déploiement
/// particulier ailleurs.
const DEFAULT_DSN: &str = "https://5c97a3f2d24e9faf2a5f099a8c5a3a80@o4504715328552960.ingest.us.sentry.io/4512093170434048";

/// La remontée d'incidents est-elle active dans cette exécution ?
pub fn telemetry_active() -> bool {
    dsn().is_some()
}

/// Attend que la file d'incidents parte, et dit si elle est partie.
///
/// [`Guard`] en fait autant à la fermeture, mais avec le budget court que
/// supportent les commandes ordinaires. Les deux appelants qui demandent plus
/// ont la même raison : ils annoncent un identifiant au joueur, et « consigné »
/// ne se dit pas comme « transmis ». Chercher dans le tableau de bord un
/// identifiant qu'une panne de réseau y a empêché d'arriver coûte plus de temps
/// que l'attente qu'on s'épargnait.
///
/// Rend `false` quand la télémétrie est coupée : rien n'est parti, faute d'avoir
/// été émis.
pub fn flush_incidents(budget: std::time::Duration) -> bool {
    sentry::Hub::current()
        .client()
        .map(|client| client.flush(Some(budget)))
        .unwrap_or(false)
}

/// La remontée d'incidents est-elle autorisée ?
///
/// Opt-out explicite : une télémétrie qu'on ne peut pas couper n'est pas une
/// télémétrie, c'est une surveillance.
fn telemetry_enabled() -> bool {
    match std::env::var("SAMFLIX_TELEMETRY") {
        Ok(v) => !matches!(v.trim(), "0" | "off" | "false" | "no" | "non"),
        Err(_) => true,
    }
}

fn dsn() -> Option<String> {
    if !telemetry_enabled() {
        return None;
    }
    let configured = std::env::var("SENTRY_DSN").unwrap_or_else(|_| DEFAULT_DSN.to_string());
    let configured = configured.trim().to_string();
    (!configured.is_empty()).then_some(configured)
}

#[cfg(test)]
#[path = "incidents.test.rs"]
mod tests;
