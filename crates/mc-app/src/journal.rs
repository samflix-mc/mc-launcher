//! Ce que le front dit au journal.
//!
//! ## Pourquoi le front écrit dans le journal de Rust
//!
//! La fenêtre n'a pas d'inspecteur en production : `console.log` y part dans le
//! vide. Or les défauts qui comptent sont des défauts de SÉQUENCE entre deux
//! fenêtres et un processus — « qui a navigué, quand, et dans laquelle » — et
//! ils ne se lisent que si les deux côtés écrivent dans le même flux, dans
//! l'ordre, avec la même horloge.
//!
//! Deux journaux séparés qu'il faut recoller à la main sont pires qu'un seul :
//! l'erreur de recollement est précisément l'erreur qu'on cherche.
//!
//! ## L'étiquette n'est PAS envoyée par le front
//!
//! Elle est lue sur la fenêtre appelante, que Tauri injecte. C'est le seul
//! point du dispositif où l'on ne peut pas se tromper de fenêtre — et se
//! tromper de fenêtre est justement le défaut que ce module sert à traquer.

use tauri::Window;

/// Ce que le front peut demander, et rien d'autre.
///
/// Fonction PURE : elle porte la seule règle du module, et c'est donc elle
/// qu'on éprouve. Un niveau inconnu vaut `info` plutôt que de perdre la ligne —
/// un message de diagnostic qui disparaît parce qu'il était mal étiqueté est le
/// contraire de ce qu'on veut ici.
fn niveau_de(brut: &str) -> tracing::Level {
    match brut {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    }
}

/// Écrit une ligne venue du front, sous l'étiquette de sa fenêtre.
///
/// La cible est `front` : `RUST_LOG=front=trace` suffit alors à ne voir que le
/// front, et `RUST_LOG=warn,front=trace` à le voir seul au milieu d'un
/// démarrage silencieux.
#[tauri::command]
pub fn journal(fenetre: Window, niveau: String, message: String) {
    let etiquette = fenetre.label().to_string();
    match niveau_de(&niveau) {
        tracing::Level::ERROR => {
            tracing::error!(target: "front", fenetre = %etiquette, "{message}")
        }
        tracing::Level::WARN => tracing::warn!(target: "front", fenetre = %etiquette, "{message}"),
        tracing::Level::DEBUG => {
            tracing::debug!(target: "front", fenetre = %etiquette, "{message}")
        }
        tracing::Level::TRACE => {
            tracing::trace!(target: "front", fenetre = %etiquette, "{message}")
        }
        tracing::Level::INFO => tracing::info!(target: "front", fenetre = %etiquette, "{message}"),
    }
}

#[cfg(test)]
#[path = "journal.test.rs"]
mod tests;
