//! Où vivent les journaux, comment ils se nomment, quand ils disparaissent.

use std::path::{Path, PathBuf};

/// Journaux conservés, en jours.
///
/// Assez pour qu'un joueur retrouve la trace d'un incident de la semaine, pas
/// assez pour que le répertoire grossisse indéfiniment.
const KEEP_DAYS: u64 = 14;

/// Répertoire des journaux.
pub fn log_dir() -> PathBuf {
    mc_dl::data_dir().join("logs")
}

/// Le nom que `rolling::daily` donne au fichier du jour.
///
/// L'appender date le nom : « mc-pack.log » devient « mc-pack.log.2026-09-16 ».
/// Annoncer le nom sans sa date envoyait le joueur vers un fichier qui n'existe
/// pas — et c'est justement celui qu'on lui demande de joindre.
///
/// La date est calculée, pas devinée en parcourant le répertoire : celui-ci
/// peut contenir un fichier daté du futur, qu'une horloge fausse a laissé et
/// que la purge ne retire jamais (`elapsed` échoue sur un horodatage à venir).
/// Il l'emporterait alors sur le nom du jour, pour toujours. C'est la même
/// horloge et le même format que l'appender : UTC, `[year]-[month]-[day]`.
pub(crate) fn current_log_name(component: &str) -> String {
    format!("{component}.log.{}", time::OffsetDateTime::now_utc().date())
}

/// Âge à partir duquel un journal est supprimé.
///
/// Le calcul est ici plutôt qu'en ligne : deux semaines écrites en jours, en
/// heures et en secondes se confondent vite, et une multiplication changée en
/// addition ferait une limite de quatre heures — les journaux disparaîtraient
/// entre deux parties, sans que rien ne le signale.
fn retention() -> std::time::Duration {
    std::time::Duration::from_secs(KEEP_DAYS * 24 * 3600)
}

/// Supprime les journaux trop anciens.
pub(crate) fn purge_old_logs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let limit = retention();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "log") || path.to_string_lossy().contains(".log") {
            let too_old = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| t.elapsed().map(|age| age > limit).unwrap_or(false))
                .unwrap_or(false);
            if too_old {
                std::fs::remove_file(&path).ok();
            }
        }
    }
}

#[cfg(test)]
#[path = "journal.test.rs"]
mod tests;
