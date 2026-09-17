//! Où vit la session entre deux lancements.
//!
//! Le contenu est l'état complet de `minecraft-auth` : il porte un jeton de
//! rafraîchissement, qui rouvre le compte du joueur sans mot de passe ni second
//! facteur. C'est le secret le plus lourd que le launcher détienne.
//!
//! Deux endroits, dans cet ordre :
//!
//! 1. **le trousseau du système** — Secret Service, Keychain, Credential
//!    Manager. Chiffré au repos, déverrouillé par la session de l'utilisateur,
//!    hors de portée d'une sauvegarde qui embarquerait `~/.config` ;
//! 2. **un fichier `0600`** — `~/.config/samflix-mc/session.json`, pour les
//!    machines qui n'ont pas de trousseau : un conteneur, une session sans
//!    portefeuille, un runner d'intégration. Le repli est journalisé en `warn` ;
//!    ce n'est pas le fonctionnement normal, et ça doit se voir.
//!
//! ## Pourquoi les deux, et pas l'un ou l'autre
//!
//! La lecture essaie le trousseau **puis** le fichier. Une session ouverte au
//! terminal avant que le trousseau ne soit disponible reste donc utilisable, et
//! l'inverse aussi — la fenêtre et la ligne de commande partagent le compte
//! sans que le joueur ait à se connecter deux fois.
//!
//! L'écriture, elle, ne touche pas au fichier tant que le trousseau répond.
//! Elle ne l'efface pas non plus : supprimer sous son nez la session qu'une
//! autre commande vient d'ouvrir serait la casser. Seul [`effacer`] — la
//! déconnexion, qui est demandée — vide les deux.
//!
//! ## Comment s'en passer
//!
//! `SAMFLIX_SANS_TROUSSEAU=1` met le trousseau hors circuit : tout passe par le
//! fichier `0600`. Deux usages, et le second n'est pas le moins important.
//!
//! Pour un poste où le portefeuille est plus une gêne qu'un service — un
//! kdewallet chiffré par GPG redemande sa phrase à chaque accès — c'est une
//! porte de sortie.
//!
//! Pour les suites, c'est une nécessité. Le fichier s'isole en déplaçant
//! `XDG_CONFIG_HOME` ; le trousseau, non : il est unique pour la session de
//! l'utilisateur. Un test qui lance `mc-auth logout` efface alors la vraie
//! session du poste qui exécute la suite — ce qui est arrivé, et se manifestait
//! par une reconnexion Microsoft exigée après chaque `cargo test`.

mod fichier;
mod trousseau;

use anyhow::Result;

pub use fichier::chemin;

/// Le trousseau est-il utilisable ?
///
/// Lu à chaque appel plutôt que mis en cache : une suite pose la variable pour
/// le sous-processus qu'elle lance, et rien ne garantit l'ordre dans lequel les
/// premiers accès se produisent.
fn trousseau_permis() -> bool {
    !matches!(
        std::env::var("SAMFLIX_SANS_TROUSSEAU").as_deref(),
        Ok("1") | Ok("true") | Ok("oui")
    )
}

/// La session enregistrée, ou `None` si personne ne s'est connecté ici.
pub fn charger() -> Option<serde_json::Value> {
    if !trousseau_permis() {
        return fichier::charger_depuis(&chemin());
    }
    match trousseau::charger() {
        Ok(Some(etat)) => Some(etat),
        // Rien dans le trousseau : reste le fichier.
        Ok(None) => fichier::charger_depuis(&chemin()),
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "trousseau illisible, lecture du fichier de session");
            fichier::charger_depuis(&chemin())
        }
    }
}

/// Écrit la session, au trousseau si la machine en a un.
pub fn enregistrer(etat: &serde_json::Value) -> Result<()> {
    if !trousseau_permis() {
        return fichier::enregistrer_dans(&chemin(), etat);
    }
    match trousseau::enregistrer(etat) {
        Ok(()) => {
            tracing::debug!("session enregistrée dans le trousseau du système");
            Ok(())
        }
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, "trousseau indisponible, repli sur le fichier 0600");
            fichier::enregistrer_dans(&chemin(), etat)
        }
    }
}

/// Oublie la session, des deux côtés.
///
/// Les deux effacements sont tentés avant de rendre la main : échouer sur l'un
/// laisserait l'autre en place, et un jeton qu'on croit supprimé est pire qu'un
/// jeton qu'on sait présent.
pub fn effacer() -> Result<()> {
    if !trousseau_permis() {
        return fichier::effacer_de(&chemin());
    }
    let trousseau = trousseau::effacer();
    let fichier = fichier::effacer_de(&chemin());
    trousseau.and(fichier)
}

#[cfg(test)]
#[path = "stockage.test.rs"]
mod tests;
