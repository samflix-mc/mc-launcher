//! Ce que l'interface a le droit d'appeler.
//!
//! Rien n'est réécrit ici : `mc-auth` authentifie, `mc-pack` installe et lance.
//! Ce module traduit — des structures sérialisables, des événements, et des
//! messages d'erreur lisibles — et pose les quelques gardes qu'une fenêtre
//! exige et qu'un terminal n'a pas besoin d'avoir.

use std::sync::atomic::{AtomicBool, Ordering};

use mc_auth::{Auth, DeviceCode, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

pub mod nouvelles;
pub mod pack;
pub mod reglages;

use crate::marque::Marque;
use crate::phase::Phase;
use crate::suivi::Suivi;

/// L'événement qui porte le code d'appareil jusqu'à la fenêtre.
///
/// `Auth::login` ne rend la main qu'une fois le joueur passé chez Microsoft :
/// le code ne peut donc pas être la valeur de retour de la commande, il faut
/// le pousser pendant l'attente.
pub const EVENEMENT_CODE: &str = "auth://code";

/// Ce que l'application garde entre deux commandes.
#[derive(Default)]
pub struct Etat {
    /// Le compteur d'avancement, partagé avec les téléchargements.
    pub suivi: std::sync::Arc<Suivi>,
    /// Une installation tourne-t-elle déjà ?
    ///
    /// Un bouton se clique deux fois, et la seconde installation écrirait dans
    /// les mêmes répertoires que la première — deux `install` concurrents sur
    /// le même verrou, c'est un pack à moitié posé. La ligne de commande n'a
    /// pas ce problème : on n'y lance pas deux fois la même commande dans le
    /// même processus.
    en_cours: AtomicBool,
}

/// Rend `en_cours` à `false` quoi qu'il arrive.
///
/// Un `?` au milieu de l'installation sortirait de la fonction sans le
/// remettre, et le bouton resterait éteint pour le reste de la session.
struct Jeton<'a>(&'a AtomicBool);

impl Drop for Jeton<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Etat {
    /// Prend le jeton d'installation, ou dit qu'il est déjà pris.
    fn reserver(&self) -> Option<Jeton<'_>> {
        self.en_cours
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Jeton(&self.en_cours))
    }
}

/// Le compte connecté, tel que la fenêtre l'affiche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compte {
    pub pseudo: String,
    pub uuid: String,
    /// Faux quand le compte n'a pas de licence Minecraft Java Edition. La
    /// connexion réussit quand même — c'est un compte Microsoft valide — mais
    /// aucun serveur en ligne ne l'acceptera, et rien ne sert d'installer huit
    /// cents mégaoctets pour l'apprendre ensuite.
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

/// Une phase du chemin, telle que la fenêtre la dessine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtapeVue {
    pub phase: Phase,
    pub libelle: String,
    pub rang: usize,
}

/// Le chemin complet, dans l'ordre.
///
/// Demandé une fois à l'ouverture. L'interface a besoin de le connaître en
/// entier **avant** que quoi que ce soit ne commence : c'est ce qui distingue
/// « on en est à la moitié » de « il se passe quelque chose ».
#[tauri::command]
pub fn chemin() -> Vec<EtapeVue> {
    Phase::TOUTES
        .iter()
        .map(|phase| EtapeVue {
            phase: *phase,
            libelle: phase.libelle().to_string(),
            rang: phase.rang(),
        })
        .collect()
}

/// Sous quel nom le launcher se présente.
///
/// Figé à la compilation par `MC_LAUNCHER_NOM` : « samflix-mc » est le nom du
/// réseau aujourd'hui, pas une constante du produit.
#[tauri::command]
pub fn marque() -> Marque {
    Marque::courante()
}

/// Le compte déjà connecté sur cette machine, s'il y en a un.
///
/// Appelée à l'ouverture de la fenêtre. L'accès rafraîchit paresseusement les
/// jetons, d'où la réécriture : sans elle, le lancement suivant repartirait du
/// jeton périmé et redemanderait un code pour rien.
#[tauri::command]
pub async fn statut() -> Result<Option<Compte>, Erreur> {
    let Some(etat) = mc_auth::charger() else {
        return Ok(None);
    };

    let auth = Auth::resume(&etat)?;
    let session = auth.session().await.map_err(|erreur| {
        erreur.context("la session enregistrée n'est plus valable — reconnecte-toi")
    })?;
    let possede_le_jeu = auth.owns_game().await?;
    mc_auth::enregistrer(&auth.etat().await?)?;

    Ok(Some(Compte::from((&session, possede_le_jeu))))
}

/// Ouvre une session Microsoft, par code d'appareil.
///
/// Le flux n'a pas de champ de mot de passe à nous : Microsoft donne un code,
/// le joueur l'autorise dans son navigateur, et l'appel attend là jusqu'à ce
/// qu'il l'ait fait — ou que le code expire.
#[tauri::command]
pub async fn connexion(app: AppHandle, etat: State<'_, Etat>) -> Result<Compte, Erreur> {
    etat.suivi.phase(Phase::Connexion);
    let auth = Auth::login({
        let app = app.clone();
        move |code| annoncer(&app, code)
    })
    .await?;

    let session = auth.session().await?;

    etat.suivi.phase(Phase::Licence);
    let possede_le_jeu = auth.owns_game().await?;
    mc_auth::enregistrer(&auth.etat().await?)?;

    etat.suivi.termine(Phase::Licence);
    tracing::info!(pseudo = %session.profile.name, "session Microsoft ouverte");
    Ok(Compte::from((&session, possede_le_jeu)))
}

/// Oublie la session.
#[tauri::command]
pub fn deconnexion(etat: State<'_, Etat>) -> Result<(), Erreur> {
    mc_auth::effacer()?;
    // Ce qui est installé reste sur le disque : c'est LUI que le bouton lit
    // désormais, et non un champ peuplé par la session courante. Un joueur qui
    // se déconnecte puis se reconnecte retrouve donc son pack posé, là où
    // l'ancienne version lui proposait de tout réinstaller.
    etat.suivi.phase(Phase::Connexion);
    tracing::info!("session oubliée");
    Ok(())
}

/// Ce que le jeu a laissé en s'arrêtant, dit en une phrase.
///
/// Fermer sa fenêtre n'est pas une panne, et un signal non plus : seul un code
/// de sortie non nul en est une. Les confondre ferait clignoter un bandeau
/// rouge à chaque fin de partie.
pub(crate) fn verdict(rapport: &mc_instance::launch::Report) -> String {
    match &rapport.outcome {
        mc_instance::launch::Outcome::Normal => "Partie terminée.".to_string(),
        mc_instance::launch::Outcome::Interrupted { .. } => "Jeu fermé.".to_string(),
        mc_instance::launch::Outcome::Failed { code } => {
            let detail = rapport
                .errors
                .first()
                .map(|erreur| format!(" — {}", erreur.exception))
                .unwrap_or_default();
            format!("Le jeu s'est arrêté sur une erreur (code {code}){detail}")
        }
    }
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
