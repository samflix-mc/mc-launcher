//! Where logs live, how they're named, when they disappear.

use std::path::{Path, PathBuf};

/// Logs kept, in days.
///
/// Enough for a player to find the trace of an incident from the past week,
/// not so much that the directory grows without bound.
const KEEP_DAYS: u64 = 14;

/// Log directory.
///
/// Derived from `mc-paths`, which places it under the data directory — and
/// not from an "app_log_dir" from the system, which would put it under
/// `~/Library/Logs` on macOS, i.e. outside what's enough to delete to start
/// over.
pub fn log_dir() -> PathBuf {
    mc_paths::current().logs
}

/// The name `rolling::daily` gives the day's file.
///
/// The appender dates the name: "mc-pack.log" becomes
/// "mc-pack.log.2026-09-16". Announcing the name without its date sent the
/// player to a file that doesn't exist — and that's exactly the one they're
/// asked to attach.
///
/// The date is computed, not guessed by scanning the directory: the latter
/// can contain a file dated in the future, left by a wrong clock, that the
/// purge never removes (`elapsed` fails on a timestamp yet to come). It
/// would then win over the day's name, forever. Same clock and same format
/// as the appender: UTC, `[year]-[month]-[day]`.
pub(crate) fn current_log_name(component: &str) -> String {
    format!("{component}.log.{}", time::OffsetDateTime::now_utc().date())
}

/// Age past which a log is deleted.
///
/// The computation lives here rather than inline: two weeks written in
/// days, in hours and in seconds are easy to confuse, and a multiplication
/// turned into an addition would make a four-hour limit — logs would
/// disappear between two sessions, with nothing to signal it.
fn retention() -> std::time::Duration {
    std::time::Duration::from_secs(KEEP_DAYS * 24 * 3600)
}

/// Deletes logs that are too old.
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
#[path = "log.test.rs"]
mod tests;
