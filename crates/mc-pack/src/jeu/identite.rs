//! Sous quelle identité on lance le jeu.
//!
//! Le choix est explicite, jamais deviné : `--pseudo` demande une session
//! hors-ligne, son absence demande le compte Microsoft enregistré. Un repli
//! silencieux de l'un vers l'autre ferait entrer un joueur sur un serveur sous
//! une identité qu'il n'a pas choisie — et, sur un serveur en ligne, un
//! refus de connexion sans cause lisible.

use anyhow::{Result, bail};

use mc_instance::launch::Session;

/// Sous quelle identité jouer.
///
/// Un `enum` plutôt qu'un `Option<String>` : le second se lit « peut-être un
/// pseudo », et l'appelant doit deviner ce que `None` veut dire. Ici les deux
/// intentions sont nommées, et aucune n'est le défaut de l'autre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identite {
    /// Le compte enregistré. Nécessaire pour les serveurs en ligne.
    Microsoft,
    /// Un profil local, sans jeton. Serveurs `online-mode=false` seulement.
    HorsLigne(String),
}

pub(crate) async fn choisir(identite: Identite) -> Result<Session> {
    match identite {
        Identite::HorsLigne(pseudo) => Ok(hors_ligne(&pseudo)),
        Identite::Microsoft => en_ligne().await,
    }
}

/// L'UUID suit la règle du serveur vanilla, donc le joueur garde le même d'une
/// partie à l'autre — inventaire, position et permissions compris.
fn hors_ligne(pseudo: &str) -> Session {
    let profil = mc_auth::offline_session(pseudo).profile;
    tracing::info!(pseudo = %profil.name, "session hors-ligne");
    Session::offline(&profil.name, &profil.id)
}

/// La session enregistrée par `mc-auth login`, rafraîchie si besoin.
async fn en_ligne() -> Result<Session> {
    let Some(etat) = mc_auth::charger() else {
        bail!(
            "aucune session enregistrée.\n\
             Se connecter avec « mc-auth login », ou jouer hors ligne avec \
             « --pseudo <NOM> »."
        );
    };

    let auth = mc_auth::Auth::resume(&etat)?;
    let session = auth.session().await.map_err(|e| {
        e.context("la session enregistrée n'est plus valable — relancer « mc-auth login »")
    })?;

    // Le rafraîchissement est paresseux : sans cette réécriture, le lancement
    // suivant repartirait du jeton périmé et redemanderait un code pour rien.
    mc_auth::enregistrer(&auth.etat().await?)?;

    tracing::info!(pseudo = %session.profile.name, "session Microsoft");
    Ok(Session::online(
        &session.profile.name,
        &session.profile.id,
        &session.minecraft_token,
    ))
}

#[cfg(test)]
#[path = "identite.test.rs"]
mod tests;
