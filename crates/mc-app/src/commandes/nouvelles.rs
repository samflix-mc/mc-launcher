//! Le fil de nouvelles, tel que la page le reçoit.

use super::Erreur;

/// Où le fil et ses images sont mis en cache.
///
/// Sous les DONNÉES et non sous la configuration : c'est reconstructible, et
/// l'effacer ne coûte qu'un rechargement. La configuration, elle, est ce qu'on
/// ne doit pas perdre.
fn cache() -> std::path::PathBuf {
    mc_chemins::courants().donnees.join("nouvelles")
}

/// Le fil, du réseau ou de la dernière copie connue.
///
/// Les images sont rapatriées ici, en Rust, et livrées en `data:` — ce qui
/// évite d'ouvrir `img-src` à un hôte distant, et de redemander l'image à
/// chaque ouverture de la page.
#[tauri::command]
pub async fn nouvelles() -> Result<mc_nouvelles::Fil, Erreur> {
    let cache = cache();
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    let mut fil = mc_nouvelles::charger(mc_pack::source::url_par_defaut(), &cache, &dl).await?;

    // Les images, dans la limite de ce qu'on s'autorise. Une image qui échoue
    // n'emporte PAS son billet : la page l'affichera sans illustration, ce qui
    // est très largement préférable à un trou dans le fil.
    let mut rapatriees = 0usize;
    for billet in &mut fil.billets {
        let Some(url) = billet.image.clone() else {
            continue;
        };
        if rapatriees >= mc_nouvelles::IMAGES_MAX {
            billet.image = None;
            continue;
        }
        match mc_nouvelles::rapatrier(&url, &cache, &dl).await {
            Ok(chemin) => match mc_nouvelles::en_data_url(&chemin) {
                Ok(data) => {
                    billet.image = Some(data);
                    rapatriees += 1;
                }
                Err(erreur) => {
                    tracing::warn!(url, erreur = %erreur, "image illisible, billet sans illustration");
                    billet.image = None;
                }
            },
            Err(erreur) => {
                tracing::warn!(url, erreur = %erreur, "image non rapatriée");
                billet.image = None;
            }
        }
    }

    Ok(fil)
}
