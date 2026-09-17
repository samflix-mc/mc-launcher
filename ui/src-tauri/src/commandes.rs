//! Ce que l'interface a le droit d'appeler.
//!
//! Rien de la chaîne d'authentification n'est réécrit ici : `mc-auth` la porte
//! déjà, testée, et ce module ne fait que la traduire en quelque chose qu'une
//! fenêtre peut afficher — des structures sérialisables, un événement pour le
//! code d'appareil, et des messages d'erreur lisibles.

use mc_auth::{Auth, DeviceCode, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use crate::coffre;

/// L'événement qui porte le code d'appareil jusqu'à la fenêtre.
///
/// `Auth::login` ne rend la main qu'une fois le joueur passé chez Microsoft :
/// le code ne peut donc pas être la valeur de retour de la commande, il faut
/// le pousser pendant l'attente.
pub const EVENEMENT_CODE: &str = "auth://code";

/// Le compte connecté, tel que la fenêtre l'affiche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compte {
    pub pseudo: String,
    pub uuid: String,
    /// Faux quand le compte n'a pas de licence Minecraft Java Edition. La
    /// connexion réussit quand même — c'est un compte Microsoft valide — mais
    /// aucun serveur en ligne ne l'acceptera, et le dire tôt évite de chercher
    /// la panne au lancement.
    pub possede_le_jeu: bool,
}

impl From<(&Session, bool)> for Compte {
    fn from((session, possede_le_jeu): (&Session, bool)) -> Self {
        Self {
            pseudo: session.profile.name.clone(),
            uuid: session.profile.id.clone(),
            possede_le_jeu,
        }
    }
}

/// Ce que le joueur doit saisir chez Microsoft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeAppareil {
    pub code: String,
    /// La page où saisir le code à la main.
    pub url: String,
    /// La même page, code prérempli. C'est celle qu'on ouvre.
    pub url_directe: String,
}

impl From<&DeviceCode> for CodeAppareil {
    fn from(code: &DeviceCode) -> Self {
        Self {
            code: code.user_code.clone(),
            url: code.verification_uri.clone(),
            url_directe: code.verification_uri_directe.clone(),
        }
    }
}

/// Une erreur, telle qu'elle traverse le pont vers la fenêtre.
///
/// `anyhow::Error` ne se sérialise pas, et n'en garder que le dernier message
/// perdrait le contexte : c'est la chaîne complète qui distingue « connexion
/// Microsoft » de « connexion Microsoft : le code a expiré ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Erreur(String);

impl From<anyhow::Error> for Erreur {
    fn from(erreur: anyhow::Error) -> Self {
        Self(
            erreur
                .chain()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" : "),
        )
    }
}

/// Le compte déjà connecté sur cette machine, s'il y en a un.
///
/// Appelée à l'ouverture de la fenêtre. L'accès rafraîchit paresseusement les
/// jetons, d'où la réécriture : sans elle, le lancement suivant repartirait du
/// jeton périmé et redemanderait un code pour rien.
#[tauri::command]
pub async fn statut() -> Result<Option<Compte>, Erreur> {
    let Some(etat) = coffre::charger() else {
        return Ok(None);
    };

    let auth = Auth::resume(&etat)?;
    let session = auth.session().await.map_err(|erreur| {
        erreur.context("la session enregistrée n'est plus valable — reconnecte-toi")
    })?;
    let possede_le_jeu = auth.owns_game().await?;
    coffre::enregistrer(&auth.etat().await?)?;

    Ok(Some(Compte::from((&session, possede_le_jeu))))
}

/// Ouvre une session Microsoft, par code d'appareil.
///
/// Le flux n'a pas de champ de mot de passe à nous : Microsoft donne un code,
/// le joueur l'autorise dans son navigateur, et l'appel attend là jusqu'à ce
/// qu'il l'ait fait — ou que le code expire. C'est ce que `mc-auth` fait déjà
/// au terminal ; ici, le code part vers la fenêtre et la page s'ouvre.
#[tauri::command]
pub async fn connexion(app: AppHandle) -> Result<Compte, Erreur> {
    let auth = Auth::login(move |code| annoncer(&app, code)).await?;

    let session = auth.session().await?;
    let possede_le_jeu = auth.owns_game().await?;
    coffre::enregistrer(&auth.etat().await?)?;

    tracing::info!(pseudo = %session.profile.name, "session Microsoft ouverte");
    Ok(Compte::from((&session, possede_le_jeu)))
}

/// Oublie la session.
#[tauri::command]
pub fn deconnexion() -> Result<(), Erreur> {
    coffre::effacer()?;
    tracing::info!("session oubliée");
    Ok(())
}

/// Ce que le bouton « Jouer » fait pour l'instant : le dire.
///
/// Le lancement existe — c'est `mc-pack launch` — mais il n'est pas branché à
/// cette fenêtre. Rendre un message plutôt qu'une erreur est délibéré : rien
/// n'a raté, la fonction n'est pas encore là, et un bandeau rouge dirait le
/// contraire.
#[tauri::command]
pub fn lancer_jeu() -> String {
    "Le lancement n'est pas encore branché à cette interface — « mc-pack launch » s'en charge."
        .to_string()
}

/// Pousse le code vers la fenêtre, et ouvre la page.
///
/// Les deux sont tentés séparément : un navigateur qui ne s'ouvre pas laisse
/// le code affiché, qui suffit à se connecter à la main. L'inverse ne serait
/// pas vrai, d'où l'ordre.
fn annoncer(app: &AppHandle, code: &DeviceCode) {
    let code = CodeAppareil::from(code);

    if let Err(erreur) = app.emit(EVENEMENT_CODE, code.clone()) {
        tracing::warn!(erreur = %erreur, "le code d'appareil n'a pas atteint la fenêtre");
    }

    if let Err(erreur) = app.opener().open_url(&code.url_directe, None::<&str>) {
        tracing::warn!(erreur = %erreur, "page Microsoft non ouverte, le code reste affiché");
    }
}

#[cfg(test)]
#[path = "commandes.test.rs"]
mod tests;
