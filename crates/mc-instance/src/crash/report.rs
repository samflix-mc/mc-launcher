//! Find the report the session just wrote.

use std::path::{Path, PathBuf};

use super::reading::{Crash, parse};

/// a three-day-old one attributed to today's launch would send things down
/// a false trail.
pub fn find(game_dir: &Path, started_at: std::time::SystemTime) -> Option<Crash> {
    latest_crash_report(game_dir, started_at)
        .and_then(|path| parse_file(&path))
        .or_else(|| {
            let log = game_dir.join("logs").join("latest.log");
            recent_enough(&log, started_at).then(|| parse_file(&log))?
        })
}

/// Most recent crash report, if it dates from this run.
fn latest_crash_report(game_dir: &Path, started_at: std::time::SystemTime) -> Option<PathBuf> {
    let dir = game_dir.join("crash-reports");
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("crash-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(modified, _)| *modified >= started_at)
        .collect();

    candidates.sort_by_key(|(modified, _)| *modified);
    candidates.pop().map(|(_, path)| path)
}

fn recent_enough(path: &Path, started_at: std::time::SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|modified| modified >= started_at)
        .unwrap_or(false)
}

fn parse_file(path: &Path) -> Option<Crash> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut crash = parse(&text)?;
    crash.source = path.to_path_buf();
    Some(crash)
}

pub fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}

#[cfg(test)]
#[path = "report.test.rs"]
mod tests;
