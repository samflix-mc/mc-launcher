//! La partie en cours, et le moyen d'y mettre fin.
//!
//! ## Pourquoi le launcher doit pouvoir tuer le jeu
//!
//! `jouer` ne rend la main qu'à la fin de la partie. C'est ce qui permet de
//! rendre un compte rendu — ce que le jeu a laissé, les erreurs relevées — mais
//! cela suppose que la partie se termine. Un Minecraft figé sur un écran de
//! chargement, un pilote graphique qui ne répond plus, une fenêtre qui ne
//! s'affiche jamais : le launcher attend alors indéfiniment, son bouton bloqué
//! sur « En jeu », et la seule issue est le gestionnaire de tâches.
//!
//! ## Un numéro de processus, et pas la poignée
//!
//! Le `Child` de tokio est emprunté par la boucle qui lit la sortie du jeu tout
//! au long de la partie ; le partager demanderait un verrou sur le chemin le
//! plus chaud de `mc-instance`. Un numéro se copie, et signaler un processus ne
//! demande rien d'autre.
//!
//! Le prix est connu et accepté : entre le moment où l'on lit le numéro et
//! celui où l'on signale, le processus a pu se terminer. Le système rend alors
//! une erreur que l'on journalise sans la remonter — il n'y a rien à annoncer à
//! quelqu'un qui voulait arrêter un jeu déjà arrêté.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::commandes::Erreur;

/// Le numéro de processus du jeu, ou zéro s'il n'y en a pas.
///
/// Zéro comme absence plutôt qu'un `Option` sous verrou : c'est écrit depuis la
/// tâche d'installation et lu depuis la commande, et aucun système n'attribue
/// le numéro zéro à un processus.
static PID: AtomicU32 = AtomicU32::new(0);

/// Le jeu vient de démarrer.
pub fn demarree(pid: u32) {
    PID.store(pid, Ordering::Release);
}

/// La partie est finie, de quelque façon que ce soit.
///
/// **Appelée sur TOUS les chemins de sortie**, y compris les erreurs : un
/// numéro laissé derrière soi serait réutilisé par le système pour un autre
/// programme, et le bouton « arrêter » tuerait alors n'importe quoi.
pub fn terminee() {
    PID.store(0, Ordering::Release);
}

/// Le numéro du jeu en cours, s'il y en a un.
pub fn en_cours() -> Option<u32> {
    match PID.load(Ordering::Acquire) {
        0 => None,
        pid => Some(pid),
    }
}

/// Arrête la partie, si elle tourne encore.
///
/// Sans ménagement : ce geste existe pour un jeu qui ne répond plus, et lui
/// demander poliment de se fermer est précisément ce qui ne marche pas dans ce
/// cas-là. Un joueur dont le jeu répond n'a pas besoin de ce bouton, il a le
/// menu du jeu.
#[tauri::command]
pub async fn arreter_le_jeu() -> Result<(), Erreur> {
    let Some(pid) = en_cours() else {
        return Err(Erreur::from(anyhow::anyhow!("aucune partie en cours")));
    };

    let (programme, arguments) = ordre_d_arret(pid);
    tracing::warn!(pid, "arrêt du jeu demandé depuis la fenêtre");

    let sortie = std::process::Command::new(programme)
        .args(&arguments)
        .output()
        .map_err(|erreur| Erreur::from(anyhow::Error::new(erreur)))?;

    if !sortie.status.success() {
        // Le cas courant est « ce processus n'existe plus » : la partie s'est
        // terminée entre le clic et l'ordre. Rien à annoncer au joueur, qui
        // voulait justement qu'elle s'arrête.
        tracing::info!(
            pid,
            code = sortie.status.code(),
            "l'ordre d'arrêt n'a rien trouvé à tuer"
        );
    }

    terminee();
    Ok(())
}

/// Comment on arrête un processus, selon le système.
///
/// Fonction PURE, séparée de l'exécution : c'est la seule partie de ce module
/// qu'on puisse éprouver sans lancer un vrai jeu, et c'est celle où une faute
/// de frappe coûte cher — un mauvais drapeau ne tue rien, et l'on ne s'en
/// aperçoit qu'avec un jeu figé sous les yeux.
///
/// `-KILL` et non `-TERM` : la JVM installe des gestionnaires pour le second,
/// et un processus figé ne les exécute pas. Sous Windows, `/T` emporte les
/// processus enfants, que la JVM crée pour son propre compte.
fn ordre_d_arret(pid: u32) -> (&'static str, Vec<String>) {
    if cfg!(windows) {
        (
            "taskkill",
            vec![
                "/PID".to_string(),
                pid.to_string(),
                "/T".to_string(),
                "/F".to_string(),
            ],
        )
    } else {
        ("kill", vec!["-KILL".to_string(), pid.to_string()])
    }
}

#[cfg(test)]
#[path = "partie.test.rs"]
mod tests;
