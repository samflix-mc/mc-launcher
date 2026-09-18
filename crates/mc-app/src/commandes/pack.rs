//! Le geste unique, et ce que la fenêtre en apprend.

use serde::Serialize;
use tauri::{AppHandle, State};

use super::{Erreur, Etat};

/// Ce qu'un geste a laissé derrière lui.
///
/// **Le même type pour les deux gestes**, et c'est délibéré : installer et
/// jouer laissent exactement les mêmes traces — des mods introuvables, des
/// écarts au verrou, une purge, un pack distant injoignable. Seul le `verdict`
/// diffère, et c'est une phrase.
///
/// Deux types jumeaux obligeraient l'écran à porter deux chemins d'affichage
/// pour dire la même chose, et la moitié la moins empruntée finirait par
/// diverger sans qu'on s'en aperçoive.
///
/// Un COMPTE RENDU et non une chaîne, et c'est une exigence du front : trois
/// champs qu'il affiche n'ont pas d'autre source. `introuvables` en
/// particulier est un résultat de résolution, lisible sur aucun disque — il
/// n'existe que dans le verrou que l'installation vient d'écrire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompteRendu {
    /// Ce que le geste a laissé, dit en une phrase.
    pub verdict: String,
    /// Une installation a-t-elle eu lieu ?
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

/// Le compte rendu d'un déroulement, avec le verdict qui va avec le geste.
///
/// Extraite parce qu'elle est appelée par les DEUX commandes : la recopier
/// laisserait l'une des deux perdre un champ le jour où l'on en ajoute un, et
/// ce genre d'oubli ne se voit qu'à l'écran, sur un cas rare.
fn compte_rendu(deroulement: &mc_pack::Deroulement, verdict: String) -> CompteRendu {
    CompteRendu {
        verdict,
        rattrapee: deroulement.installation.is_some(),
        introuvables: deroulement.introuvables(),
        ecarts: deroulement.ecarts(),
        hors_ligne: deroulement.etat.hors_ligne,
        purge: deroulement
            .installation
            .as_ref()
            .map(|pose| pose.purge.vides.clone())
            .unwrap_or_default(),
    }
}

/// Ce que le disque et le pack publié disent, sans rien installer.
///
/// Appelée à l'ouverture de Spawn, et c'est elle qui décide de ce que le
/// bouton affiche. Quelques dizaines de kilooctets de réseau : le verrou seul.
#[tauri::command]
pub async fn etat_du_pack() -> Result<mc_pack::EtatDuPack, Erreur> {
    Ok(crate::cinematique::etat_du_pack().await?)
}

/// Pose le pack, et **s'arrête là**.
///
/// ## Le bouton ne lance plus le jeu tout seul
///
/// Il le faisait : vérifier, rattraper, puis démarrer Minecraft, d'un seul
/// clic. Sam l'a repris là-dessus à la recette — « ça lance le jeu alors qu'on
/// voulait juste installer le modpack » — et le motif est net : poser huit
/// cents mégaoctets et jouer sont deux intentions, et la seconde ne se déduit
/// pas de la première.
///
/// Ce que le geste unique avait résolu n'est pas perdu : la vérification reste
/// en tête des deux chemins, et personne n'attend un téléchargement pour jouer
/// à un pack déjà à jour.
///
/// Le jeton est le même que celui de [`jouer`] : une installation et une partie
/// écriraient dans les mêmes répertoires.
#[tauri::command]
pub async fn installer(app: AppHandle, etat: State<'_, Etat>) -> Result<CompteRendu, Erreur> {
    let Some(_jeton) = etat.reserver() else {
        return Err(Erreur("une opération est déjà en cours".to_string()));
    };

    let deroulement = crate::cinematique::mettre_a_jour(&app, &etat.suivi).await?;

    let verdict = if deroulement.installation.is_some() {
        "Le pack est installé.".to_string()
    } else {
        "Le pack était déjà à jour : rien à poser.".to_string()
    };
    Ok(compte_rendu(&deroulement, verdict))
}

/// LE bouton, quand il dit JOUER.
///
/// Vérifie le pack publié, rattrape ce qui a bougé s'il y a lieu, puis lance
/// la partie. Ne rend la main qu'à la fin de celle-ci.
///
/// La vérification reste en tête : la retirer ferait entrer le joueur avec des
/// registres NeoForge qui ne concordent plus, ce qui se manifeste par une
/// éjection à la connexion sans message utile.
///
/// Le jeton d'installation est pris ici, et sa raison a changé : il ne protège
/// plus contre deux installations concurrentes seulement, mais contre deux
/// PARTIES — deux `mettre_a_jour_et_jouer` écriraient dans les mêmes
/// répertoires et lanceraient deux jeux sur la même instance.
#[tauri::command]
pub async fn jouer(app: AppHandle, etat: State<'_, Etat>) -> Result<CompteRendu, Erreur> {
    let Some(_jeton) = etat.reserver() else {
        return Err(Erreur("une opération est déjà en cours".to_string()));
    };

    let deroulement = crate::cinematique::mettre_a_jour_et_jouer(&app, &etat.suivi).await?;

    let verdict = deroulement
        .partie
        .as_ref()
        .map(super::verdict)
        .unwrap_or_else(|| "Partie terminée.".to_string());
    Ok(compte_rendu(&deroulement, verdict))
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
