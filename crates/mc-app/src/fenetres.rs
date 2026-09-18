//! La fenêtre de connexion, et le passage qu'elle ouvre.
//!
//! ## Pourquoi une FENÊTRE et non une modale
//!
//! La connexion était un dialogue par-dessus la fenêtre principale. Ça marchait
//! et ça n'allait pas : la fenêtre principale fait mille vingt-quatre pixels de
//! large parce qu'elle a une image à montrer et trois sections à porter, et une
//! modale de quatre cent vingt pixels posée au milieu de ce cadre a l'air d'une
//! boîte oubliée sur un bureau vide.
//!
//! Le design system le dit autrement, et mieux : il décrit **trois fenêtres** —
//! l'écran de démarrage en 360 × 480, la connexion en 440 × 520, la principale
//! en 1024 × 640 au minimum. Chacune fait la taille de ce qu'elle porte.
//!
//! ## Le chemin que le launcher suit
//!
//! ```text
//!   splash ──┬── session trouvée ──→ main
//!            └── aucune session  ──→ connexion ──→ main
//! ```
//!
//! La fenêtre principale est CRÉÉE dans les deux cas — elle est déclarée dans
//! `tauri.conf.json` avec `visible: false` — mais elle ne se montre que par un
//! de ces deux chemins. C'est ce qui permet au front de faire sa poignée de
//! main réseau pendant que le joueur regarde l'écran de démarrage.
//!
//! ## Fermer la connexion QUITTE le launcher
//!
//! Et c'est la seule chose raisonnable : la fenêtre principale est cachée, il
//! n'y a rien derrière, et un processus qui survit à sa dernière fenêtre
//! visible est un processus qu'on ne retrouve que dans le gestionnaire de
//! tâches. La garde ne joue que tant que la connexion n'a pas abouti — après,
//! c'est `connexion_reussie` qui ferme la fenêtre, et le drapeau est baissé.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commandes::Erreur;

/// L'étiquette de la fenêtre de connexion.
pub const ETIQUETTE: &str = "connexion";

/// La route que la fenêtre charge.
///
/// La MÊME application Angular, à une autre adresse. Le front reconnaît cette
/// route et se dessine en conséquence : une feuille dépolie pleine fenêtre, une
/// barre de titre sans bouton d'agrandissement, et le dialogue au milieu.
pub(crate) const ROUTE: &str = "/connexion";

/// Les dimensions du design system, à la ligne près.
pub(crate) const LARGEUR: f64 = 440.0;
pub(crate) const HAUTEUR: f64 = 520.0;

/// L'événement par lequel la fenêtre principale apprend qu'elle a une session.
///
/// Elle a chargé son front AVANT la connexion, et son service de session porte
/// donc un compte nul. Sans ce signal, elle se montrerait sur une page de
/// connexion qu'elle n'a plus de raison d'afficher, et il faudrait relancer le
/// launcher pour en sortir.
pub const EVENEMENT_SESSION: &str = "session-ouverte";

/// La connexion est-elle en cours ?
///
/// Lu par `demarrage::accomplir` : tant qu'il est levé, refermer l'écran de
/// démarrage ne doit PAS montrer la fenêtre principale — ce serait la montrer
/// vide, sur une page de connexion qui vit ailleurs, le temps que le joueur
/// s'authentifie.
static EN_COURS: AtomicBool = AtomicBool::new(false);

/// Vrai tant que le joueur n'a pas de session et que la fenêtre dédiée est là.
pub fn connexion_en_cours() -> bool {
    EN_COURS.load(Ordering::Acquire)
}

/// Ouvre la fenêtre de connexion, et efface la principale.
///
/// Appelée par le front de la fenêtre principale, dès qu'il sait que la session
/// n'est pas jouable. Idempotente : si la fenêtre existe déjà, on lui rend le
/// focus plutôt que d'en créer une seconde — deux fenêtres de connexion
/// demanderaient deux codes d'appareil à Microsoft, et seule l'une des deux
/// aboutirait.
#[tauri::command]
pub async fn ouvrir_connexion(app: AppHandle) -> Result<(), Erreur> {
    EN_COURS.store(true, Ordering::Release);

    if let Some(principale) = app.get_webview_window("main")
        && let Err(erreur) = principale.hide()
    {
        tracing::warn!(erreur = %erreur, "la fenêtre principale n'a pas pu s'effacer");
    }

    if let Some(deja) = app.get_webview_window(ETIQUETTE) {
        deja.set_focus().map_err(en_erreur)?;
        return Ok(());
    }

    let fenetre = WebviewWindowBuilder::new(&app, ETIQUETTE, WebviewUrl::App(ROUTE.into()))
        .title("Connexion")
        .inner_size(LARGEUR, HAUTEUR)
        .center()
        .decorations(false)
        // Ni redimensionnable ni agrandissable : le design system pose les deux,
        // et la règle du système fait ce que le gabarit ne peut pas faire seul —
        // cacher le bouton d'agrandissement ne l'empêcherait pas d'exister au
        // double-clic sur la barre.
        .resizable(false)
        .maximizable(false)
        .minimizable(true)
        .closable(true)
        .background_color(tauri::window::Color(0x16, 0x14, 0x11, 0xff))
        .build()
        .map_err(en_erreur)?;

    // Fermer la connexion quitte le launcher : la fenêtre principale est
    // cachée, il n'y a rien derrière, et un processus qui survit à sa dernière
    // fenêtre visible ne se retrouve que dans le gestionnaire de tâches.
    let sortie = app.clone();
    fenetre.on_window_event(move |evenement| {
        if matches!(evenement, tauri::WindowEvent::Destroyed) && connexion_en_cours() {
            tracing::info!("fenêtre de connexion fermée sans session : le launcher s'arrête");
            sortie.exit(0);
        }
    });

    Ok(())
}

/// La session est ouverte : la principale reprend la main, la connexion s'en va.
///
/// L'ordre compte. On montre AVANT de fermer : l'inverse laisserait, le temps
/// d'une image, zéro fenêtre visible — ce que certains gestionnaires de bureau
/// traitent comme une application qui se termine, en retirant son entrée de la
/// barre des tâches.
#[tauri::command]
pub async fn connexion_reussie(app: AppHandle) -> Result<(), Erreur> {
    // Baissé d'abord : c'est lui qui désarme la garde de fermeture, et la
    // fermeture arrive deux lignes plus bas.
    EN_COURS.store(false, Ordering::Release);

    if let Some(principale) = app.get_webview_window("main") {
        principale.show().map_err(en_erreur)?;
        if let Err(erreur) = principale.set_focus() {
            tracing::warn!(erreur = %erreur, "focus non donné à la fenêtre principale");
        }
        // Elle a chargé son front avant la connexion : son service de session
        // porte un compte nul, et rien ne le lui dirait sans ce signal.
        if let Err(erreur) = app.emit_to("main", EVENEMENT_SESSION, ()) {
            tracing::warn!(erreur = %erreur, "la fenêtre principale n'a pas reçu le signal de session");
        }
    } else {
        tracing::error!("aucune fenêtre « main » à montrer après la connexion");
    }

    if let Some(fenetre) = app.get_webview_window(ETIQUETTE)
        && let Err(erreur) = fenetre.close()
    {
        tracing::warn!(erreur = %erreur, "fenêtre de connexion non refermée");
    }

    Ok(())
}

/// Une erreur de Tauri, dans la forme que le pont sait rendre.
fn en_erreur(erreur: tauri::Error) -> Erreur {
    Erreur::from(anyhow::Error::new(erreur))
}

#[cfg(test)]
#[path = "fenetres.test.rs"]
mod tests;
