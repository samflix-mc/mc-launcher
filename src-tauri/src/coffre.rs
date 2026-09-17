//! Où vit la session entre deux lancements.
//!
//! Le contenu est l'état complet de `minecraft-auth` : il porte un jeton de
//! rafraîchissement, qui rouvre le compte du joueur sans mot de passe ni second
//! facteur. C'est le secret le plus lourd que le launcher détienne.
//!
//! Il va donc au **trousseau du système** — Secret Service sous Linux, Keychain
//! sous macOS, Credential Manager sous Windows. Chiffré au repos, déverrouillé
//! par la session de l'utilisateur, hors de portée d'une sauvegarde qui
//! embarquerait `~/.config` ou d'un `cat` malheureux.
//!
//! ## Quand la machine n'en a pas
//!
//! Un conteneur, une session sans portefeuille : il n'y a alors rien à quoi
//! parler. Refuser de se connecter là serait pire que le fichier. Le repli est
//! celui de la ligne de commande — `mc-auth`, qui écrit en `0600` dans
//! `~/.config/samflix-mc/session.json`. Il est journalisé en `warn` : ce n'est
//! pas le fonctionnement normal, et ça doit se voir.
//!
//! ## Cohabitation avec la ligne de commande
//!
//! `mc-auth login` écrit ce fichier et `mc-pack launch` le lit. La lecture
//! essaie donc le trousseau **puis** le fichier : une session ouverte au
//! terminal est reprise ici sans rien redemander. L'écriture, elle, ne touche
//! pas au fichier tant que le trousseau répond — mais ne l'efface pas non
//! plus : supprimer la session que la ligne de commande vient d'ouvrir serait
//! la casser dans son dos. Seule la déconnexion, qui est demandée, vide les
//! deux.

use anyhow::{Context, Result};
use serde_json::Value;

/// Le service, tel qu'il apparaît dans le trousseau.
const SERVICE: &str = "samflix-mc";

/// L'entrée. Il n'y en a qu'une : le launcher ne connaît qu'un compte à la
/// fois, exactement comme `mc-auth`.
const ENTREE: &str = "session-minecraft";

/// La session enregistrée, ou `None` si personne ne s'est connecté ici.
pub fn charger() -> Option<Value> {
    match lire_trousseau() {
        Ok(Some(etat)) => Some(etat),
        // Rien dans le trousseau : reste le fichier, qu'une connexion au
        // terminal a pu écrire.
        Ok(None) => mc_auth::charger(),
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "trousseau illisible, lecture du fichier de session");
            mc_auth::charger()
        }
    }
}

/// Écrit la session, au trousseau si la machine en a un.
pub fn enregistrer(etat: &Value) -> Result<()> {
    let brut = serde_json::to_string(etat).context("sérialisation de la session")?;
    match ecrire_trousseau(&brut) {
        Ok(()) => {
            tracing::debug!("session enregistrée dans le trousseau du système");
            Ok(())
        }
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "trousseau indisponible, repli sur le fichier 0600");
            mc_auth::enregistrer(etat)
        }
    }
}

/// Oublie la session, des deux côtés.
///
/// Les deux effacements sont tentés avant de rendre la main : échouer sur l'un
/// laisserait l'autre en place, et un jeton qu'on croit supprimé est pire
/// qu'un jeton qu'on sait présent.
pub fn effacer() -> Result<()> {
    let trousseau = vider_trousseau();
    let fichier = mc_auth::effacer();
    trousseau.and(fichier)
}

fn lire_trousseau() -> Result<Option<Value>> {
    let entree = keyring::Entry::new(SERVICE, ENTREE).context("ouverture du trousseau")?;
    let brut = match entree.get_password() {
        Ok(brut) => brut,
        Err(erreur) if absente(&erreur) => return Ok(None),
        Err(erreur) => return Err(anyhow::Error::new(erreur).context("lecture du trousseau")),
    };

    // Un secret présent mais illisible vaut « pas de session » : l'appelant
    // proposera de se reconnecter, là où une erreur fatale empêcherait de
    // jouer à cause d'une entrée corrompue. Le contenu n'est pas journalisé.
    match serde_json::from_str(&brut) {
        Ok(etat) => Ok(Some(etat)),
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "session du trousseau illisible, connexion à refaire");
            Ok(None)
        }
    }
}

fn ecrire_trousseau(brut: &str) -> Result<()> {
    let entree = keyring::Entry::new(SERVICE, ENTREE).context("ouverture du trousseau")?;
    entree
        .set_password(brut)
        .context("écriture dans le trousseau")
}

fn vider_trousseau() -> Result<()> {
    let entree = match keyring::Entry::new(SERVICE, ENTREE) {
        Ok(entree) => entree,
        // Pas de trousseau du tout : il n'y a rien à y oublier, et le fichier
        // reste à effacer. Ce n'est pas une erreur.
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "trousseau indisponible, rien à y effacer");
            return Ok(());
        }
    };
    match entree.delete_credential() {
        Ok(()) => Ok(()),
        Err(erreur) if absente(&erreur) => Ok(()),
        Err(erreur) => Err(anyhow::Error::new(erreur).context("effacement dans le trousseau")),
    }
}

/// L'entrée n'existe pas — ce qui n'est pas une panne.
///
/// La distinction porte tout le module : « rien d'enregistré » mène au
/// fichier, « le trousseau est verrouillé » doit se voir dans le journal.
fn absente(erreur: &keyring::Error) -> bool {
    matches!(erreur, keyring::Error::NoEntry)
}

#[cfg(test)]
#[path = "coffre.test.rs"]
mod tests;
