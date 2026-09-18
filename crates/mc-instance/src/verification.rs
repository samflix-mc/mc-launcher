//! Verify that an installation is complete and intact.

use anyhow::Result;

use crate::layout::Layout;
use crate::{neoforge, vanilla};

/// Verifies that an installation is complete and intact.
///
/// `deep` re-checks the digest of every asset object, which the installer
/// skips so it doesn't re-read 800 MB on every launch.
pub fn verify(
    mc: &str,
    neoforge_version: &str,
    layout: &Layout,
    deep: bool,
) -> Result<Vec<String>> {
    let shared = layout.shared();
    let mut problems = Vec::new();

    let version_json = shared.join("versions").join(mc).join(format!("{mc}.json"));
    let client_jar = shared.join("versions").join(mc).join(format!("{mc}.jar"));
    for path in [&version_json, &client_jar] {
        if !path.is_file() {
            problems.push(format!("missing file: {}", path.display()));
        }
    }

    let neoforge_json = shared
        .join("versions")
        .join(neoforge::version_id(neoforge_version))
        .join(format!("{}.json", neoforge::version_id(neoforge_version)));
    if !neoforge_json.is_file() {
        problems.push(format!(
            "NeoForge {neoforge_version} is not installed: {} missing",
            neoforge_json.display()
        ));
    }

    // Both descriptors are checked: NeoForge's adds about fifty libraries
    // to the classpath, and missing just one fails the startup as surely
    // as a missing vanilla library would.
    for descriptor in [&version_json, &neoforge_json] {
        if !descriptor.is_file() {
            continue;
        }
        for library in vanilla::classpath(descriptor, &shared)? {
            if !library.is_file() {
                problems.push(format!("missing library: {}", library.display()));
            }
        }
    }

    if deep && problems.is_empty() {
        let index = shared.join("assets").join("indexes");
        let id = std::fs::read_dir(&index)
            .ok()
            .and_then(|entries| {
                entries.flatten().find_map(|e| {
                    e.path()
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
            })
            .unwrap_or_default();
        if !id.is_empty() {
            let report = vanilla::verify_assets(&shared, &id)?;
            for hash in report.missing {
                problems.push(format!("missing asset: {hash}"));
            }
            for hash in report.corrupt {
                problems.push(format!("corrupt asset: {hash}"));
            }
        }
    }

    Ok(problems)
}

#[cfg(test)]
#[path = "verification.test.rs"]
mod tests;
