//! La façade sur `minecraft-auth`.
//!
//! Le gestionnaire de la bibliothèque est **avec état** : il garde les jetons
//! de chaque étage et les rafraîchit paresseusement quand on y accède. C'est
//! l'inverse de l'ancienne chaîne écrite à la main, qui repartait de zéro à
//! chaque appel — et c'est précisément ce qu'on vient chercher : une session
//! reprise d'un lancement à l'autre sans redemander de code.

use anyhow::{Context, Result};
use minecraft_auth::java::JavaAuthManager;

mod code;

pub use code::DeviceCode;

pub struct Auth {
    manager: JavaAuthManager,
}

impl Auth {
    /// Ouvre une session par *device code*.
    ///
    /// `on_code` reçoit de quoi guider l'utilisateur, puis l'appel bloque
    /// jusqu'à ce qu'il ait autorisé — ou que le code expire.
    pub async fn login(on_code: impl FnOnce(&DeviceCode)) -> Result<Self> {
        let manager = JavaAuthManager::builder(client()?)
            .login_device_code(|code| {
                on_code(&DeviceCode {
                    user_code: code.user_code.clone(),
                    verification_uri: code.verification_uri.clone(),
                    verification_uri_directe: code.direct_verification_uri(),
                });
            })
            .await
            .context("connexion Microsoft")?;
        Ok(Self { manager })
    }

    /// Reprend une session persistée, sans redemander de code.
    ///
    /// Ne contacte personne : les jetons ne seront rafraîchis qu'au premier
    /// accès qui en a besoin.
    pub fn resume(etat: &serde_json::Value) -> Result<Self> {
        let manager =
            JavaAuthManager::from_json(client()?, etat).context("session enregistrée illisible")?;
        Ok(Self { manager })
    }

    /// L'état complet, à réenregistrer après chaque usage.
    ///
    /// Le rafraîchissement étant paresseux, l'état d'après un appel n'est pas
    /// celui d'avant : ne pas le réécrire ferait repartir du jeton périmé au
    /// lancement suivant, et redemander un code pour rien.
    pub async fn etat(&self) -> Result<serde_json::Value> {
        self.manager
            .to_json()
            .await
            .context("sérialisation de la session")
    }

    /// Le joueur et son jeton, prêts pour la ligne de commande du jeu.
    pub async fn session(&self) -> Result<crate::Session> {
        let session = self
            .manager
            .launch_session()
            .await
            .context("ouverture de la session de jeu")?;
        Ok(crate::Session {
            minecraft_token: session.access_token,
            profile: crate::Profile {
                // Sans tirets, comme `offline_session` et comme Prism : c'est
                // la forme que le jeu attend.
                id: session.player_uuid.simple().to_string(),
                name: session.player_name,
            },
        })
    }

    /// Le compte possède-t-il le jeu ? Un inventaire vide vaut « non ».
    pub async fn owns_game(&self) -> Result<bool> {
        let ents = self
            .manager
            .entitlements()
            .await
            .context("vérification de la licence")?;
        Ok(!ents.items.is_empty())
    }
}

/// Le client HTTP des appels d'authentification.
///
/// `minecraft-auth` pose son propre délai d'expiration par requête ; celui-ci
/// ne sert qu'à identifier le launcher et à fixer la pile TLS.
fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("samflix-mc-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("construction du client HTTP")
}
