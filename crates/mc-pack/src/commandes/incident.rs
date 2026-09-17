//! Remonter ce qui s'est passé du côté du jeu.

use mc_pack::lockfile::Lockfile;

/// Cherche la cause d'un plantage du jeu et la remonte.
///
/// Sans cela, un incident ne porterait que « code de sortie 1 » : le launcher
/// et le jeu sont deux processus, et la trace Java reste du côté du jeu. C'est
/// pourtant le seul moment où elle est disponible — le joueur, lui, ne
/// l'enverra pas.
pub(super) fn report_game_crash(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    started_at: std::time::SystemTime,
    code: i32,
) {
    let Some(crash) = mc_instance::crash::find(&instance.game_dir, started_at) else {
        // Aucune trace exploitable : l'incident du launcher reste, et il dit
        // au moins où chercher.
        tracing::warn!(
            code,
            journaux = %instance.game_dir.join("logs").display(),
            "Plantage sans trace exploitable dans les journaux du jeu"
        );
        return;
    };

    let id = report_game_error(instance, lock, version_id, &crash, Some(code));
    eprintln!("\n{} : {}", crash.exception, crash.message);
    eprintln!("  relevé dans {}", crash.source.display());
    if mc_log::telemetry_active() {
        // Le seul endroit qui vaille qu'on attende le réseau : c'est le seul où
        // un identifiant est donné au joueur, et un identifiant qu'on lui
        // demandera de citer doit désigner quelque chose qui est arrivé. Le
        // verdict est dit plutôt que supposé — la fermeture n'attend que deux
        // secondes, et ce qui n'est pas parti d'ici là est perdu sans un mot.
        if mc_log::flush_incidents(std::time::Duration::from_secs(10)) {
            eprintln!("  incident transmis sous l'identifiant {id}");
        } else {
            eprintln!("  incident consigné sous l'identifiant {id}, mais non transmis");
            eprintln!("  (réseau indisponible) — joindre le journal du launcher");
        }
    }
}

/// Transmet une exception du jeu, qu'elle l'ait arrêté ou non.
///
/// Le contexte joint est celui qu'on demanderait sinon au joueur : versions,
/// mods présents, et d'où vient la trace.
pub(super) fn report_game_error(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    crash: &mc_instance::crash::Crash,
    code: Option<i32>,
) -> sentry::types::Uuid {
    let mods = mc_instance::crash::loaded_mods(&instance.game_dir).unwrap_or_default();
    let mut contexte = std::collections::BTreeMap::from([
        ("version".to_string(), version_id.to_string()),
        ("minecraft".to_string(), lock.minecraft.clone()),
        ("neoforge".to_string(), lock.loader.version.clone()),
        ("source".to_string(), crash.source.display().to_string()),
        ("mods".to_string(), mods.join("\n")),
    ]);
    if let Some(code) = code {
        contexte.insert("code_sortie".to_string(), code.to_string());
    } else {
        // Sans quoi rien ne distinguerait, dans le tableau de bord, une erreur
        // traversée d'une erreur fatale.
        contexte.insert("fatale".to_string(), "non".to_string());
    }

    tracing::error!(
        exception = %crash.exception,
        source = %crash.source.display(),
        fatale = code.is_some(),
        "Minecraft : {} : {}",
        crash.exception,
        crash.message
    );

    mc_log::capture_game_crash(&crash.exception, &crash.message, &crash.excerpt, &contexte)
}

#[cfg(test)]
#[path = "incident.test.rs"]
mod tests;
