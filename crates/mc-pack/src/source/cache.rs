//! Where the copy of a remote pack is stored, and how the address is named.

use std::path::PathBuf;

pub(super) fn is_url(arg: &str) -> bool {
    let lowered = arg.to_ascii_lowercase();
    lowered.starts_with("https://") || lowered.starts_with("http://")
}

/// `…/samflix.json` gives `…/samflix.lock.json`, same as on disk.
pub fn lock_url_for(url: &str) -> String {
    match url.strip_suffix(".json") {
        Some(base) => format!("{base}.lock.json"),
        None => format!("{url}.lock.json"),
    }
}

pub(super) fn file_name_of(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    match path.rsplit('/').next() {
        Some(name) if name.ends_with(".json") => name.to_string(),
        _ => "pack.json".to_string(),
    }
}

/// The cache is organized by host, not flat.
///
/// Dev and production manifests share the same file name; flat, switching
/// from one to the other would silently overwrite the first, and a network
/// outage would surface the pack from the wrong environment.
pub(super) fn cache_dir_for(url: &str, layout: &mc_instance::Layout) -> PathBuf {
    let host = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .filter(|h| !h.is_empty())
        .unwrap_or("unknown");
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    layout.cache().join("packs").join(safe)
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod tests;
