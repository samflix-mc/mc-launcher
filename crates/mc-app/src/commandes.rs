//! Ce que l'interface a le droit d'appeler.
//!
//! Rien n'est réécrit ici : `mc-auth` authentifie, `mc-pack` installe et lance.
//! Ce module traduit — des structures sérialisables, des événements, et des
//! messages d'erreur lisibles — et pose les quelques gardes qu'une fenêtre
//! exige et qu'un terminal n'a pas besoin d'avoir.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use mc_auth::{Auth, DeviceCode, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::cinematique;
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
    /// Ce qu'a produit la dernière installation réussie. Tant qu'il est vide,
    /// le jeu ne peut pas être lancé — et le bouton reste éteint.
    pub installation: Mutex<Option<Installation>>,
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

/// Ce qu'une installation a posé, pour le compte rendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub instance: String,
    pub minecraft: String,
    pub neoforge: String,
    pub java: String,
    pub mods: usize,
    /// Les mods que la résolution n'a pas trouvés. Le pack s'installe quand
    /// même, mais NeoForge refusera de démarrer s'ils lui manquent : c'est à
    /// dire tout de suite, pas au premier lancement.
    pub introuvables: Vec<String>,
    /// Ce par quoi l'installation s'écarte du verrou publié. Vide quand les
    /// deux coïncident un pour un.
    pub ecarts: Vec<String>,
    /// Le pack distant était injoignable et la copie locale a servi : ce que
    /// le joueur installe peut ne plus correspondre aux serveurs.
    pub hors_ligne: bool,
}

impl From<&mc_pack::Outcome> for Installation {
    fn from(outcome: &mc_pack::Outcome) -> Self {
        Self {
            instance: outcome.instance.name.clone(),
            minecraft: outcome.lock.minecraft.clone(),
            neoforge: outcome.neoforge.clone(),
            java: outcome.java.version.full.clone(),
            mods: outcome.client_mods,
            introuvables: outcome
                .lock
                .unresolved
                .iter()
                .map(|manque| format!("{} (exigé par {})", manque.mod_id, manque.required_by))
                .collect(),
            ecarts: outcome.ecarts.clone(),
            hors_ligne: outcome.from_cache,
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
    // Ce qui a été installé reste sur le disque, mais plus personne n'est
    // connecté pour le lancer : laisser le bouton allumé promettrait une
    // partie qu'aucune session ne peut ouvrir.
    *etat.installation.lock().expect("installation") = None;
    etat.suivi.phase(Phase::Connexion);
    tracing::info!("session oubliée");
    Ok(())
}

/// Installe le pack — la partie longue.
///
/// Rend la main quand tout est en place. L'avancement ne passe pas par la
/// valeur de retour mais par l'événement de la cinématique, émis cinq fois par
/// seconde pendant tout ce temps.
#[tauri::command]
pub async fn installer(app: AppHandle, etat: State<'_, Etat>) -> Result<Installation, Erreur> {
    let Some(_jeton) = etat.reserver() else {
        return Err(Erreur("une installation est déjà en cours".to_string()));
    };

    let outcome = cinematique::installer(&app, &etat.suivi).await?;

    let installation = Installation::from(&outcome);
    *etat.installation.lock().expect("installation") = Some(installation.clone());

    tracing::info!(
        instance = %installation.instance,
        mods = installation.mods,
        introuvables = installation.introuvables.len(),
        "pack installé"
    );
    Ok(installation)
}

/// Ce que la dernière installation de cette session a posé, s'il y en a eu une.
#[tauri::command]
pub fn installation(etat: State<'_, Etat>) -> Option<Installation> {
    etat.installation.lock().expect("installation").clone()
}

/// Lance le jeu avec le compte connecté.
///
/// N'installe rien : `docs/lancement.md` pose que les deux gestes restent
/// séparés, et les enchaîner ferait attendre huit cents mégaoctets à qui
/// voulait seulement jouer.
#[tauri::command]
pub async fn lancer_jeu(app: AppHandle, etat: State<'_, Etat>) -> Result<String, Erreur> {
    let Some(_jeton) = etat.reserver() else {
        return Err(Erreur("une installation est en cours".to_string()));
    };

    let rapport = cinematique::jouer(&app, &etat.suivi).await?;
    Ok(verdict(&rapport))
}

/// Ce que le jeu a laissé en s'arrêtant, dit en une phrase.
///
/// Fermer sa fenêtre n'est pas une panne, et un signal non plus : seul un code
/// de sortie non nul en est une. Les confondre ferait clignoter un bandeau
/// rouge à chaque fin de partie.
fn verdict(rapport: &mc_instance::launch::Report) -> String {
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
