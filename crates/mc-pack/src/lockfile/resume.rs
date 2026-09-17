//! Ce qu'un verrou redemande, et ce qui a changé depuis le précédent.

use mc_mods::Side;

use super::Lockfile;

impl Lockfile {
    /// Reconstruit les demandes à partir du verrou : chaque mod est épinglé sur
    /// le build exact qui avait été retenu, dépendances comprises.
    pub fn requests(&self) -> Vec<mc_mods::Request> {
        self.mods
            .iter()
            .map(|m| mc_mods::Request {
                slug: m.project.clone(),
                source: Some(m.origin),
                file: Some(m.file.clone()),
                version: None,
                side: Side::parse(&m.side),
                channel: None,
                // Rend vérifiable un build venu d'une source qui ne publie pas
                // d'empreinte : celle-ci a été calculée au premier passage.
                expected_sha1: m.sha1.clone(),
                expected_sha512: m.sha512.clone(),
            })
            .collect()
    }

    /// Ce qui a changé par rapport à un autre verrou, en clair.
    pub fn diff(&self, previous: &Lockfile) -> Vec<String> {
        let mut lines = Vec::new();
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
#[path = "resume.test.rs"]
mod tests;
