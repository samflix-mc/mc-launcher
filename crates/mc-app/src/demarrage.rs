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
//! ## Les deux bornes, et elles ne servent pas à la même chose
//!
//! Un **plancher** ([`DUREE_MINIMALE`]) : en dessous, l'écran de démarrage
//! passe trop vite pour être lu, et ce qui devait ressembler à une intention
//! ressemble à un défaut d'affichage.
//!
//! Un **plafond** ([`DELAI_DE_GARDE`]) : si le front n'appelle jamais, la
//! fenêtre principale resterait cachée POUR TOUJOURS derrière un écran sans le
//! moindre bouton, et la seule issue serait de tuer le processus.
//!
//! Les deux se mesurent depuis le même instant — celui où les fenêtres
//! existent — et c'est pourquoi ils vivent ici plutôt que dans le front : le
//! front ne sait pas quand l'écran de démarrage est apparu, il ne sait que
//! quand lui-même a fini.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

/// Combien de temps l'écran de démarrage reste affiché AU MINIMUM.
///
/// Mesuré sur ce poste, le front signale son premier rendu au bout d'environ
/// six cents millisecondes. C'est assez court pour qu'on n'ait le temps de
/// rien lire : l'écran apparaît et disparaît, ce qui se remarque comme un
/// clignotement et non comme un démarrage.
///
/// Une seconde et demie est le seuil au-delà duquel une attente cesse d'être
/// perçue comme un raté et devient une transition. C'est un choix de rythme,
/// pas une contrainte technique — et il est donc écrit ici, seul, pour qu'on
/// puisse le changer sans rien relire d'autre.
const DUREE_MINIMALE: Duration = Duration::from_millis(1500);

/// Au bout de combien de temps on montre la fenêtre sans attendre le front.
///
/// Dix secondes : bien au-delà de tout démarrage réel, y compris sur un disque
/// lent au premier lancement, et bien en deçà de ce qu'un joueur accepterait
/// de regarder sans rien comprendre.
const DELAI_DE_GARDE: Duration = Duration::from_secs(10);

/// L'instant où les fenêtres ont été créées.
///
/// L'origine des deux bornes. Posé dans le hook `setup`, qui s'exécute quand
/// les fenêtres de `tauri.conf.json` existent réellement — c'est-à-dire à
/// quelques millisecondes près le moment où l'écran de démarrage devient
/// visible.
static DEPART: OnceLock<Instant> = OnceLock::new();

/// Le passage n'a lieu qu'une fois.
///
/// La garde et le front peuvent arriver ensemble — un front lent exactement à
/// la dixième seconde. Sans ce verrou, on fermerait deux fois l'écran de
/// démarrage, ce qui journalise une erreur pour rien.
static PASSAGE_FAIT: AtomicBool = AtomicBool::new(false);

/// Ce qu'il reste à attendre avant d'avoir le droit de montrer la fenêtre.
///
/// Fonction PURE, séparée de l'attente : c'est elle qui porte la règle, et
/// c'est donc elle qu'on peut éprouver sans horloge ni fenêtre.
fn reste_a_attendre(ecoule: Duration) -> Duration {
    DUREE_MINIMALE.saturating_sub(ecoule)
}

/// Réserve le passage, ou dit qu'il a déjà eu lieu.
///
/// Le drapeau est pris AVANT l'attente du plancher, et non après. Sinon, deux
/// appelants arrivés pendant cette attente la traverseraient tous les deux et
/// se retrouveraient à montrer la fenêtre en même temps.
fn reserver() -> bool {
    !PASSAGE_FAIT.swap(true, Ordering::AcqRel)
}

/// Montre la fenêtre principale et ferme l'écran de démarrage.
fn accomplir(app: &AppHandle, pourquoi: &str) {
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
///
/// Asynchrone parce qu'elle peut ATTENDRE : si le front a été plus rapide que
/// le plancher, on tient l'écran de démarrage le temps qui reste. Le front,
/// lui, n'attend rien d'utile de cette promesse — il l'ignore.
#[tauri::command]
pub async fn front_pret(app: AppHandle) {
    if !reserver() {
        return;
    }

    let ecoule = DEPART.get().map(Instant::elapsed).unwrap_or_default();
    let reste = reste_a_attendre(ecoule);
    if !reste.is_zero() {
        tracing::debug!(
            rendu_en_ms = ecoule.as_millis(),
            attente_ms = reste.as_millis(),
            "front prêt avant le plancher : l'écran de démarrage est tenu"
        );
        tokio::time::sleep(reste).await;
    }

    accomplir(&app, "le front a signalé son premier rendu");
}

/// Arme la garde de délai et pose l'origine des deux bornes.
///
/// À appeler dans le hook `setup` : c'est le premier instant où les fenêtres
/// existent réellement.
pub fn armer_la_garde(app: &AppHandle) {
    // `set` plutôt que `get_or_init` : une seconde pose serait un bogue de
    // séquencement, et l'ignorer en silence nous ferait mesurer le plancher
    // depuis le mauvais instant.
    if DEPART.set(Instant::now()).is_err() {
        tracing::warn!("l'origine du démarrage était déjà posée");
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DELAI_DE_GARDE).await;
        if !reserver() {
            return;
        }
        tracing::warn!(
            secondes = DELAI_DE_GARDE.as_secs(),
            "le front n'a rien signalé : la fenêtre est montrée quand même. \
             Chercher une erreur JavaScript dans la console du build --debug."
        );
        accomplir(&app, "délai de garde écoulé");
    });
}

#[cfg(test)]
#[path = "demarrage.test.rs"]
mod tests;
