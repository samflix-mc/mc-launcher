//! What a lockfile requests again, and what changed since the previous one.

use mc_mods::Side;

use super::Lockfile;

impl Lockfile {
    /// Rebuilds the requests from the lockfile: each mod is pinned to the
    /// exact build that had been retained, dependencies included.
    pub fn requests(&self) -> Vec<mc_mods::Request> {
        self.mods
            .iter()
            .map(|m| mc_mods::Request {
                slug: m.project.clone(),
                source: Some(m.source),
                file: Some(m.file.clone()),
                version: None,
                side: Side::parse(&m.side),
                channel: None,
                // Makes a build from a source that doesn't publish a digest
                // verifiable: this one was computed on the first pass.
                expected_sha1: m.sha1.clone(),
                expected_sha512: m.sha512.clone(),
            })
            .collect()
    }

    /// What changed compared to another lockfile, in plain terms.
    pub fn diff(&self, previous: &Lockfile) -> Vec<String> {
        let mut lines = Vec::new();

        // The generation first, and spelled out: it's the only line of the
        // diff that describes a consequence and not a fact. Whoever
        // publishes must see, in review, that they just asked every player
        // to erase their mods — not infer it from a lost “0 → 1” among
        // thirty lines of versions.
        if self.generation != previous.generation {
            lines.push(format!(
                "! generation {} → {} — machines already installed will erase \
                 mods, shaders and resource packs before reinstalling \
                 (saves and configurations kept)",
                previous.generation, self.generation
            ));
        }

        for current in &self.mods {
            match previous.mods.iter().find(|p| p.project == current.project) {
                None => lines.push(format!("+ {} {}", current.slug, current.version)),
                Some(before) if before.file != current.file => lines.push(format!(
                    "~ {} {} → {}",
                    current.slug, before.version, current.version
                )),
                Some(_) => {}
            }
        }
        for before in &previous.mods {
            if !self.mods.iter().any(|c| c.project == before.project) {
                lines.push(format!("- {} {}", before.slug, before.version));
            }
        }
        lines
    }
}

#[cfg(test)]
#[path = "summary.test.rs"]
mod tests;
