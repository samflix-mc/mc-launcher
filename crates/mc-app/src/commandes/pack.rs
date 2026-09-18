//! Le geste unique, et ce que la fenêtre en apprend.

use serde::Serialize;
use tauri::{AppHandle, State};

use super::{Erreur, Etat};

/// Ce qu'une partie a laissé en s'arrêtant.
///
/// Un COMPTE RENDU et non une chaîne, et c'est une exigence du front : trois
/// champs qu'il affiche n'ont pas d'autre source. `introuvables` en
/// particulier est un résultat de résolution, lisible sur aucun disque — il
/// n'existe que dans le verrou que l'installation vient d'écrire.
///
/// Une chaîne obligerait la fenêtre à relire un message pour en tirer une
/// liste, ce qui est exactement ce qu'on ne veut pas d'un pont.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Partie {
    /// Ce que le jeu a laissé, dit en une phrase.
    pub verdict: String,
    /// Une installation a-t-elle eu lieu avant la partie ?
    pub rattrapee: bool,
    /// Les mods que la résolution n'a pas trouvés. Le pack s'installe quand
    /// même, mais NeoForge refusera de démarrer s'ils lui manquent.
    pub introuvables: Vec<String>,
    /// Ce par quoi l'installation s'est écartée du verrou publié.
    pub ecarts: Vec<String>,
    /// Le pack distant était injoignable : ce qui a été posé peut ne plus
    /// correspondre aux serveurs.
    pub hors_ligne: bool,
    /// Ce que la purge a effacé, s'il y a eu purge.
    pub purge: Vec<String>,
}

/// Ce que le disque et le pack publié disent, sans rien installer.
///
/// Appelée à l'ouverture de Spawn, et c'est elle qui décide de ce que le
/// bouton affiche. Quelques dizaines de kilooctets de réseau : le verrou seul.
#[tauri::command]
pub async fn etat_du_pack() -> Result<mc_pack::EtatDuPack, Erreur> {
    Ok(crate::cinematique::etat_du_pack().await?)
}

/// LE bouton.
///
/// Vérifie le pack publié, rattrape ce qui a bougé s'il y a lieu, puis lance
/// la partie. Ne rend la main qu'à la fin de celle-ci.
///
/// Le jeton d'installation est pris ici, et sa raison a changé : il ne protège
/// plus contre deux installations concurrentes seulement, mais contre deux
/// PARTIES — deux `mettre_a_jour_et_jouer` écriraient dans les mêmes
/// répertoires et lanceraient deux jeux sur la même instance.
#[tauri::command]
pub async fn jouer(app: AppHandle, etat: State<'_, Etat>) -> Result<Partie, Erreur> {
    let Some(_jeton) = etat.reserver() else {
        return Err(Erreur("une partie est déjà en cours".to_string()));
    };

    let deroulement = crate::cinematique::mettre_a_jour_et_jouer(&app, &etat.suivi).await?;

    Ok(Partie {
        verdict: deroulement
            .partie
            .as_ref()
            .map(super::verdict)
            .unwrap_or_else(|| "Partie terminée.".to_string()),
        rattrapee: deroulement.installation.is_some(),
        introuvables: deroulement.introuvables(),
        ecarts: deroulement.ecarts(),
        hors_ligne: deroulement.etat.hors_ligne,
        purge: deroulement
            .installation
            .as_ref()
            .map(|pose| pose.purge.vides.clone())
            .unwrap_or_default(),
    })
}

/// Vérifie les fichiers de l'instance posée.
///
/// Le geste de la section « Avancé ». Rend la liste des problèmes trouvés,
/// vide quand tout va bien — et non un booléen : « trois fichiers manquent »
/// et « tout va bien » ne se disent pas de la même façon.
#[tauri::command]
pub async fn verifier_les_fichiers(profond: bool) -> Result<Vec<String>, Erreur> {
    // Sur un exécuteur dédié : la vérification profonde relit et rehache
    // plusieurs centaines de mégaoctets, et la tenir sur l'exécuteur principal
    // figerait la boucle qui rafraîchit la fenêtre.
    tauri::async_runtime::spawn_blocking(move || crate::cinematique::verifier(profond))
        .await
        .map_err(|erreur| Erreur(format!("vérification interrompue : {erreur}")))?
        .map_err(Erreur::from)
}
