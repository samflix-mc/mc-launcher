//! Où l'on range la copie d'un pack distant, et comment on nomme l'adresse.

use std::path::PathBuf;

pub(super) fn is_url(arg: &str) -> bool {
    let lowered = arg.to_ascii_lowercase();
    lowered.starts_with("https://") || lowered.starts_with("http://")
}

/// `…/samflix.json` donne `…/samflix.lock.json`, comme sur le disque.
pub(super) fn lock_url_for(url: &str) -> String {
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

/// Le cache est rangé par hôte, et non à plat.
///
/// Les manifestes de dev et de production portent le même nom de fichier ; à
/// plat, passer de l'un à l'autre écraserait silencieusement le premier, et une
/// panne de réseau ressortirait le pack du mauvais environnement.
pub(super) fn cache_dir_for(url: &str, layout: &mc_instance::Layout) -> PathBuf {
    let host = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .filter(|h| !h.is_empty())
        .unwrap_or("inconnu");
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
