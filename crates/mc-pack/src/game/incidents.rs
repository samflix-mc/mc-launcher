//! Surface what happened on the game's side.

use crate::lockfile::Lockfile;

/// Looks for the cause of a game crash and reports it.
///
/// Without this, an incident would carry only "exit code 1": the launcher
/// and the game are two processes, and the Java trace stays on the game's
/// side. Yet that's the only moment it's available — the player, for their
/// part, won't send it.
pub(crate) fn report_game_crash(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    started_at: std::time::SystemTime,
    code: i32,
) {
    let Some(crash) = mc_instance::crash::find(&instance.game_dir, started_at) else {
        // No usable trace: the launcher's incident stands, and it at least
        // says where to look.
        tracing::warn!(
            code,
            logs = %instance.game_dir.join("logs").display(),
            "Crash with no usable trace in the game's logs"
        );
        return;
    };

    let id = report_game_error(instance, lock, version_id, &crash, Some(code));
    eprintln!("\n{} : {}", crash.exception, crash.message);
    eprintln!("  found in {}", crash.source.display());
    if mc_log::telemetry_active() {
        // The only place worth waiting on the network for: it's the only one
        // where an identifier is given to the player, and an identifier
        // we'll ask them to cite must point to something that actually
        // happened. The verdict is stated rather than assumed — the shutdown
        // only waits two seconds, and whatever hasn't left by then is lost
        // without a word.
        if mc_log::flush_incidents(std::time::Duration::from_secs(10)) {
            eprintln!("  incident sent under id {id}");
        } else {
            eprintln!("  incident logged under id {id}, but not sent");
            eprintln!("  (network unavailable) — attach the launcher's log");
        }
    }
}

/// Reports an exception from the game, whether it stopped it or not.
///
/// The context attached is what we'd otherwise ask the player for: versions,
/// mods present, and where the trace comes from.
pub(crate) fn report_game_error(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    crash: &mc_instance::crash::Crash,
    code: Option<i32>,
) -> sentry::types::Uuid {
    let mods = mc_instance::crash::loaded_mods(&instance.game_dir).unwrap_or_default();
    let mut context = std::collections::BTreeMap::from([
        ("version".to_string(), version_id.to_string()),
        ("minecraft".to_string(), lock.minecraft.clone()),
        ("neoforge".to_string(), lock.loader.version.clone()),
        ("source".to_string(), crash.source.display().to_string()),
        ("mods".to_string(), mods.join("\n")),
    ]);
    if let Some(code) = code {
        context.insert("exit_code".to_string(), code.to_string());
    } else {
        // Without this, nothing would distinguish, in the dashboard, a
        // caught error from a fatal one.
        context.insert("fatal".to_string(), "no".to_string());
    }

    tracing::error!(
        exception = %crash.exception,
        source = %crash.source.display(),
        fatal = code.is_some(),
        "Minecraft: {} : {}",
        crash.exception,
        crash.message
    );

    mc_log::capture_game_crash(&crash.exception, &crash.message, &crash.excerpt, &context)
}

#[cfg(test)]
#[path = "incidents.test.rs"]
mod tests;
