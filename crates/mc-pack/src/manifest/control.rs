//! What a manifest must satisfy to be installable.

use anyhow::{Result, bail};

use super::{Manifest, SCHEMA};

impl Manifest {
    pub(super) fn check(&self) -> Result<()> {
        if self.schema != SCHEMA {
            bail!(
                "manifest in format {} while this version reads format {SCHEMA}",
                self.schema
            );
        }
        if self.loader.kind != "neoforge" {
            bail!(
                "only the neoforge loader is supported, manifest: “{}”",
                self.loader.kind
            );
        }
        // `name` designates an instance directory: it's joined to the data
        // root, and `deploy` then deletes every .jar it finds in the
        // resulting folder. As long as the manifest came from a file edited
        // by hand, the field was trusted; since the remote pack became the
        // default source, it comes from the network.
        //
        // A “..” climbs up, and an absolute path does worse: PathBuf::join
        // simply discards the prefix. A compromised pack host or a typo
        // would then get a .jar deletion and a file write wherever it
        // wants, on every installation.
        //
        // We only reject what falls outside the directory. No character
        // whitelist: rejecting here what's merely unusual would doom a
        // future pack across every launcher already distributed, and a
        // launcher that rejects the pack can no longer be repaired — you'd
        // have to download the very pack it refuses to read.
        let name = self.name.trim();
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
        {
            bail!(
                "“{}” cannot name an instance directory: the pack would \
                 install outside the data root",
                self.name
            );
        }

        let mut seen = std::collections::BTreeSet::new();
        for entry in &self.mods {
            if !seen.insert(entry.slug.to_ascii_lowercase()) {
                bail!("{} appears twice in the manifest", entry.slug);
            }
            if entry.file.is_some() && entry.version.is_some() {
                bail!(
                    "{} pins both a build (file) and a version number: \
                     the two would contradict each other",
                    entry.slug
                );
            }
        }

        // The keys of `servers` aren't checked here: see server_problems,
        // and the reason that check can't live in check.
        Ok(())
    }
}

#[cfg(test)]
#[path = "control.test.rs"]
mod tests;
