//! Le trousseau du système, quand la machine en a un.
//!
//! L'état d'authentification porte un jeton de rafraîchissement : il rouvre le
//! compte du joueur sans mot de passe ni second facteur. C'est le secret le
//! plus lourd que le launcher détienne.
//!
//! Il va donc au trousseau — Secret Service sous Linux, Keychain sous macOS,
//! Credential Manager sous Windows. Chiffré au repos, déverrouillé par la
//! session de l'utilisateur, hors de portée d'une sauvegarde qui embarquerait
//! `~/.config` ou d'un `cat` malheureux.
//!
//! ## Quand la machine n'en a pas
//!
//! Un conteneur, une session sans portefeuille, un serveur d'intégration : il
//! n'y a alors rien à quoi parler. Refuser de se connecter là serait pire que
//! le fichier, et c'est [`super::fichier`] qui prend le relais — en `0600`, et
//! avec un `warn` dans le journal. Ce n'est pas le fonctionnement normal, et ça
//! doit se voir.

use anyhow::{Context, Result};
use serde_json::Value;

/// Le service, tel qu'il apparaît dans le trousseau.
const SERVICE: &str = "samflix-mc";

/// L'entrée. Il n'y en a qu'une : le launcher ne connaît qu'un compte à la
/// fois.
const ENTREE: &str = "session-minecraft";

/// La session enregistrée dans le trousseau.
///
/// `Ok(None)` veut dire « rien d'enregistré ici » — l'appelant ira voir le
/// fichier. Une `Err` veut dire que le trousseau existe mais n'a pas répondu,
/// ce qui n'est pas la même chose et doit se lire dans le journal.
/// Hors de portée des tests de mutation : cette fonction parle au trousseau du
/// système. Sur un runner il n'y en a pas, et sur un poste de développement le
/// test écrirait dans le portefeuille de l'utilisateur — c'est précisément ce
/// que le test `le_trousseau_garde_ce_qu_on_lui_confie` évite en étant
/// `#[ignore]` et en se donnant un service à lui.
///
/// La décision qu'elle porte, elle, est éprouvée : `absente` distingue « rien
/// d'enregistré » de « le trousseau n'a pas répondu », et deux tests la
/// tiennent.
#[mutants::skip]
pub(super) fn charger() -> Result<Option<Value>> {
    let entree = keyring::Entry::new(SERVICE, ENTREE).context("ouverture du trousseau")?;
    let brut = match entree.get_password() {
        Ok(brut) => brut,
        Err(erreur) if absente(&erreur) => return Ok(None),
        Err(erreur) => return Err(anyhow::Error::new(erreur).context("lecture du trousseau")),
    };

    // Un secret présent mais illisible vaut « pas de session » : l'appelant
    // proposera de se reconnecter, là où une erreur fatale empêcherait de jouer
    // à cause d'une entrée corrompue. Le contenu n'est pas journalisé.
    match serde_json::from_str(&brut) {
        Ok(etat) => Ok(Some(etat)),
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "session du trousseau illisible, connexion à refaire");
            Ok(None)
        }
    }
}

pub(super) fn enregistrer(etat: &Value) -> Result<()> {
    let brut = serde_json::to_string(etat).context("sérialisation de la session")?;
    let entree = keyring::Entry::new(SERVICE, ENTREE).context("ouverture du trousseau")?;
    entree
        .set_password(&brut)
        .context("écriture dans le trousseau")
}

/// Hors de portée des tests de mutation : cette fonction parle au trousseau du
/// système. Sur un runner il n'y en a pas, et sur un poste de développement le
/// test écrirait dans le portefeuille de l'utilisateur — c'est précisément ce
/// que le test `le_trousseau_garde_ce_qu_on_lui_confie` évite en étant
/// `#[ignore]` et en se donnant un service à lui.
///
/// La décision qu'elle porte, elle, est éprouvée : `absente` distingue « rien
/// d'enregistré » de « le trousseau n'a pas répondu », et deux tests la
/// tiennent.
#[mutants::skip]
pub(super) fn effacer() -> Result<()> {
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
/// La distinction porte tout le module : « rien d'enregistré » mène au fichier,
/// « le trousseau est verrouillé » doit se voir dans le journal.
fn absente(erreur: &keyring::Error) -> bool {
    matches!(erreur, keyring::Error::NoEntry)
}

#[cfg(test)]
#[path = "trousseau.test.rs"]
mod tests;
