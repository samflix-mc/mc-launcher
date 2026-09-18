//! Le passage de l'écran de démarrage à la fenêtre.
//!
//! ## Pourquoi DEUX fenêtres
//!
//! La fenêtre principale charge Angular : un module d'environ cent trente
//! kilooctets à analyser, compiler et exécuter avant le premier pixel. Pendant
//! ce temps, une WebView peint sa propre couleur de fond et rien d'autre.
//!
//! `backgroundColor` a supprimé le BLANC de cette attente, mais pas l'attente
//! elle-même : on regardait un rectangle sombre et vide. Une amorce écrite
//! dans `index.html` ne change rien au problème — elle appartient au même
//! document, et ne s'affiche donc pas plus tôt que lui.
//!
//! La seule façon de montrer quelque chose PENDANT ce temps est une seconde
//! fenêtre, avec son propre document, minuscule et sans script. Elle s'affiche
//! en quelques dizaines de millisecondes ; la principale reste cachée et
//! travaille.
//!
//! ## Qui décide que c'est fini
//!
//! Le front, et lui seul : c'est le seul à savoir quand il a rendu. Il appelle
//! [`front_pret`] après son premier rendu, plus un court délai — voir
//! `app.ts`. Rust ne peut pas le deviner : `RuntimeRunEvent::Ready` dit que la
//! fenêtre EXISTE, pas que son contenu est peint.
//!
//! ## La garde, et pourquoi elle n'est pas facultative
//!
//! Si le front n'appelle jamais — une erreur JavaScript, un morceau paresseux
//! introuvable, une exception dans un constructeur — la fenêtre principale
//! resterait cachée POUR TOUJOURS, et l'écran de démarrage resterait affiché
//! sans un seul bouton pour le fermer. Le launcher serait inutilisable, et la
//! seule issue serait de tuer le processus.
//!
//! Passé le délai, on montre donc la fenêtre quoi qu'il arrive. Elle affichera
//! peut-être une page cassée — mais une page cassée avec une barre de titre se
//! ferme, se signale, et se diagnostique par la console.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

/// Au bout de combien de temps on montre la fenêtre sans attendre le front.
///
/// Dix secondes : bien au-delà de tout démarrage réel, y compris sur un disque
/// lent au premier lancement, et bien en deçà de ce qu'un joueur accepterait
/// de regarder sans rien comprendre.
const DELAI_DE_GARDE: Duration = Duration::from_secs(10);

/// Le passage n'a lieu qu'une fois.
///
/// La garde et le front peuvent arriver ensemble — un front lent exactement à
/// la dixième seconde. Sans ce verrou, on fermerait deux fois l'écran de
/// démarrage, ce qui journalise une erreur pour rien.
static PASSAGE_FAIT: AtomicBool = AtomicBool::new(false);

/// Montre la fenêtre principale et ferme l'écran de démarrage.
///
/// Idempotente : les appels suivants ne font rien.
fn passer_la_main(app: &AppHandle, pourquoi: &str) {
    if PASSAGE_FAIT.swap(true, Ordering::AcqRel) {
        return;
    }

    tracing::info!(pourquoi, "écran de démarrage refermé");

    // La principale D'ABORD, l'écran de démarrage ENSUITE.
    //
    // L'ordre inverse laisserait, entre les deux appels, un instant où aucune
    // fenêtre n'est visible : sous certains gestionnaires, cela fait passer le
    // focus à une autre application, et le launcher s'ouvrirait derrière.
    if let Some(principale) = app.get_webview_window("main") {
        if let Err(erreur) = principale.show() {
            tracing::error!(erreur = %erreur, "la fenêtre principale n'a pas pu s'afficher");
        }
        if let Err(erreur) = principale.set_focus() {
            tracing::warn!(erreur = %erreur, "focus non donné à la fenêtre");
        }
    } else {
        tracing::error!("aucune fenêtre « main » à montrer");
    }

    if let Some(demarrage) = app.get_webview_window("splash")
        && let Err(erreur) = demarrage.close()
    {
        tracing::warn!(erreur = %erreur, "écran de démarrage non refermé");
    }
}

/// Le front a fini de se rendre.
///
/// Appelée par `app.ts` après son premier rendu. C'est le seul signal fiable :
/// Rust sait quand la fenêtre existe, pas quand son contenu est peint.
#[tauri::command]
pub fn front_pret(app: AppHandle) {
    passer_la_main(&app, "le front a signalé son premier rendu");
}

/// Arme la garde de délai. À appeler dans le hook `setup`.
pub fn armer_la_garde(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DELAI_DE_GARDE).await;
        if !PASSAGE_FAIT.load(Ordering::Acquire) {
            tracing::warn!(
                secondes = DELAI_DE_GARDE.as_secs(),
                "le front n'a rien signalé : la fenêtre est montrée quand même. \
                 Chercher une erreur JavaScript dans la console du build --debug."
            );
        }
        passer_la_main(&app, "délai de garde écoulé");
    });
}

#[cfg(test)]
#[path = "demarrage.test.rs"]
mod tests;
