//! Où la fenêtre a le droit d'aller.
//!
//! ## Ce qu'on empêche
//!
//! Le launcher affiche des nouvelles rédigées ailleurs, et un lien y suffirait
//! à faire naviguer la fenêtre entière vers un site distant. Cette fenêtre
//! n'est pas un navigateur : c'est une origine privilégiée où `invoke` est
//! joignable, avec accès au système de fichiers, au trousseau et au jeton du
//! joueur. Un site distant qui s'y chargerait hériterait de tout cela.
//!
//! Les liens vers l'extérieur passent donc par `ouvrirPage()`, qui les confie
//! au navigateur du système — un processus séparé qui ne connaît rien du
//! launcher.
//!
//! ## Pourquoi un greffon, et pas autre chose
//!
//! `on_navigation` n'existe pas sur `tauri::Builder`. Il existe sur deux
//! choses seulement : un constructeur de webview, et un constructeur de
//! greffon (`tauri/src/plugin.rs:458`).
//!
//! Le premier ne convient pas : notre fenêtre est déclarée dans
//! `tauri.conf.json` et n'existe pas encore au moment où l'on construirait
//! l'application. La reconstruire en code pour lui accrocher un gestionnaire
//! serait faux deux fois — elle n'existe pas après `build()`, et c'est inutile,
//! puisque le magasin de greffons est consulté pour TOUTE webview, y compris
//! celle du fichier de configuration (`manager/webview.rs:596-602`).
//!
//! ## Le danger de ce contrôle
//!
//! Trop strict, il rend une fenêtre blanche SANS UN MOT : la page initiale
//! elle-même passe par ce gestionnaire. C'est pourquoi chaque refus se
//! journalise avec son URL — un refus muet se diagnostique en ouvrant le code,
//! un refus journalisé se diagnostique en lisant le journal.

// `tauri::Url` est une réexportation de `url::Url` (tauri/src/lib.rs:83). On
// passe par elle plutôt que d'ajouter la dépendance : le type est alors
// littéralement celui que `on_navigation` reçoit, et non un homonyme d'une
// autre version qui ne s'unifierait pas.
use tauri::Url;

/// Les origines qui sont l'application elle-même.
///
/// Trois, et non une : l'origine du protocole d'actifs n'est pas la même
/// partout. `tauri://localhost` sous Linux et macOS,
/// `http://tauri.localhost` sous Windows, et `http://localhost:1420` pendant
/// un `tauri dev`, où c'est le serveur d'Angular qui sert la page.
///
/// En oublier une donne une fenêtre blanche sur la plateforme oubliée
/// seulement — c'est-à-dire, en pratique, chez quelqu'un d'autre.
const ORIGINES: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
    "http://localhost:1420",
];

/// La navigation vers cette URL est-elle celle de l'application ?
///
/// Fonction pure, et c'est tout l'intérêt : elle s'éprouve sans fenêtre, sans
/// serveur d'affichage et sans Tauri.
pub fn autorisee(url: &Url) -> bool {
    let origine = url.origin().ascii_serialization();

    // `origin()` d'une URL `tauri://localhost` rend « null » : le schéma n'est
    // pas spécial au sens de la norme URL. On compare donc aussi la forme
    // textuelle, tronquée de son chemin.
    if ORIGINES.contains(&origine.as_str()) {
        return true;
    }

    match url.host_str() {
        Some(hote) => {
            let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
            let recompose = format!("{}://{hote}{port}", url.scheme());
            ORIGINES.contains(&recompose.as_str())
        }
        None => false,
    }
}

/// Le greffon qui accroche le prédicat à toute webview de l'application.
pub fn greffon<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation")
        .on_navigation(|_fenetre, url| {
            let permis = autorisee(url);
            if !permis {
                // La seule chose qui distingue « le lien a été bloqué, c'est
                // normal » de « la fenêtre est blanche et je ne sais pas
                // pourquoi ».
                tracing::warn!(url = %url, "navigation refusée : hors de l'application");
            }
            permis
        })
        .build()
}

#[cfg(test)]
#[path = "navigation.test.rs"]
mod tests;
