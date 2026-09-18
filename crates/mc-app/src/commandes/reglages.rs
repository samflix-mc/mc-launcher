//! Lire et écrire les réglages, et ce que l'écran permet.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::Erreur;

/// Ce que l'écran du joueur permet, pour que la page propose des tailles qui
/// tiennent dessus.
// Pas d'`Eq` : le facteur d'échelle est un flottant, et l'égalité y est
// précisément ce qu'on ne veut pas dériver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ecran {
    /// La ZONE UTILE, panneaux du bureau déduits — et non la taille brute.
    ///
    /// C'est ce que « maximisée » veut dire : une fenêtre de la taille de
    /// l'écran passerait sous la barre des tâches, et le joueur ne verrait
    /// jamais le bas de son inventaire.
    pub largeur: u32,
    pub hauteur: u32,
    /// Le facteur d'échelle. Sans lui, proposer « 1920×1080 » sur un écran
    /// HiDPI donnerait une fenêtre deux fois trop petite.
    pub echelle: f64,
}

/// Ce que l'écran du joueur permet.
///
/// `current_monitor` et non `primary_monitor` : sous Wayland, la notion
/// d'écran principal n'existe pas toujours, et ce qui compte est l'écran où la
/// fenêtre se trouve. Les deux échecs se traitent de la même façon — on ne
/// propose alors aucune taille calculée, et la page retombe sur sa liste fixe.
#[tauri::command]
pub fn ecran(app: AppHandle) -> Option<Ecran> {
    let fenetre = app.get_webview_window("main")?;
    let moniteur = fenetre
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| fenetre.primary_monitor().ok().flatten())?;

    let zone = moniteur.work_area();
    Some(Ecran {
        largeur: zone.size.width,
        hauteur: zone.size.height,
        echelle: moniteur.scale_factor(),
    })
}

/// Les réglages en vigueur.
#[tauri::command]
pub fn reglages() -> mc_reglages::Reglages {
    mc_reglages::charger(&mc_reglages::chemin())
}

/// Enregistre les réglages, et rend CE QUI A ÉTÉ ÉCRIT.
///
/// La distinction compte : si une valeur a été ramenée dans ses bornes, la
/// fenêtre doit le montrer tout de suite. Rendre l'entrée laisserait un
/// curseur à une position que le fichier ne porte pas, et le joueur croirait
/// avoir réglé 200 là où le jeu en recevra 32.
#[tauri::command]
pub fn enregistrer_reglages(
    reglages: mc_reglages::Reglages,
) -> Result<mc_reglages::Reglages, Erreur> {
    Ok(mc_reglages::enregistrer(&mc_reglages::chemin(), &reglages)?)
}

/// Les dossiers qu'on propose d'ouvrir depuis la section « Avancé ».
///
/// Une énumération fermée et NON un chemin : une commande qui accepterait un
/// chemin du front permettrait d'ouvrir n'importe quoi sur la machine, depuis
/// une page dont le contenu vient en partie d'un hôte distant.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dossier {
    Donnees,
    Config,
    Journaux,
    Instance,
}

/// Ouvre un des dossiers du launcher dans l'explorateur du système.
///
/// Exige la permission `opener:allow-open-path`, qui ne figure PAS dans
/// `core:default` : sans elle, l'appel échoue sur un refus d'ACL qu'on ne voit
/// qu'en build packagé — jamais en `tauri dev`.
#[tauri::command]
pub fn ouvrir_dossier(app: AppHandle, quoi: Dossier) -> Result<(), Erreur> {
    use tauri_plugin_opener::OpenerExt;

    let emplacements = mc_chemins::courants();
    let chemin = match quoi {
        Dossier::Donnees => emplacements.donnees.clone(),
        Dossier::Config => emplacements.config.clone(),
        Dossier::Journaux => emplacements.journaux.clone(),
        Dossier::Instance => {
            let options = mc_pack::Options::default();
            options
                .layout
                .instance(mc_pack::etat::NOM_PAR_DEFAUT)
                .game_dir
        }
    };

    // Créé au besoin : ouvrir un dossier qui n'existe pas encore ouvrirait une
    // fenêtre vide de l'explorateur, ou rien du tout selon le système.
    if let Err(erreur) = std::fs::create_dir_all(&chemin) {
        tracing::warn!(erreur = %erreur, chemin = %chemin.display(), "dossier non créé");
    }

    app.opener()
        .open_path(chemin.to_string_lossy(), None::<&str>)
        .map_err(|erreur| Erreur(format!("dossier non ouvert : {erreur}")))
}
