//! Does what was placed match what the lock announced?
//!
//! Replaying a published lock means obeying it. It still has to be checked
//! that it was actually achieved: between the lock and the instance, there
//! is a resolution, downloads, sources that may have withdrawn a build, and
//! a deduplication by `modId` that can drop one jar in favor of another.
//!
//! None of this raises an error — the installation finishes "fine" — and
//! the drift only shows up when the game starts, as a NeoForge that refuses
//! to load, or worse, an ejection from the server over diverging mod lists.
//! An ejection never names its cause.
//!
//! ## What the comparison is about, and why not the slug
//!
//! It's about the **project**, identified by its source and its id — and
//! the value compared is the **build**, the version id the lock pins.
//!
//! The slug doesn't work, and that's the trap this function fell into: the
//! lock names a CurseForge mod by its readable slug (`fix-gpu-memory-leak`),
//! while the replay requests it again by its numeric id (`882495`), which
//! the candidate reuses as is. Compared by slug, these two are two
//! different mods — one missing, the other extra — and every CurseForge mod
//! in the pack produced two imaginary drifts.
//!
//! The project id, on the other hand, is the same on both sides, no matter
//! how the mod was requested.

use crate::lockfile::Lockfile;
use mc_mods::Origin;

/// A mod actually placed: where it comes from, which project, which build.
pub(super) type Placement<'a> = (Origin, &'a str, &'a str);

/// The drifts between the replayed lock and what the resolution actually
/// settled on. Empty when the two coincide one for one.
///
/// Takes `Placement` rather than a `Plan`: the comparison only looks at
/// these three fields, and an `Installed` cannot be constructed outside of
/// `mc-mods` — the function would be untestable.
pub(super) fn drifts<'a>(
    expected: &Lockfile,
    placements: impl Iterator<Item = Placement<'a>>,
) -> Vec<String> {
    let placements: std::collections::BTreeMap<(Origin, &str), &str> = placements
        .map(|(origin, project, build)| ((origin, project), build))
        .collect();

    let mut drifts = Vec::new();

    for required in &expected.mods {
        match placements.get(&(required.source, required.project.as_str())) {
            None => drifts.push(format!("{} : pinned by the lock, missing", required.slug)),
            Some(placed) if *placed != required.file => drifts.push(format!(
                "{} : build {} expected, {placed} placed",
                required.slug, required.file
            )),
            Some(_) => {}
        }
    }

    // The reverse matters just as much: a mod the lock doesn't announce is
    // a mod the servers don't have. NeoForge negotiates its registries at
    // connect time, and an extra jar fails the negotiation just as surely
    // as a missing one.
    let expected_projects: std::collections::BTreeSet<(Origin, &str)> = expected
        .mods
        .iter()
        .map(|m| (m.source, m.project.as_str()))
        .collect();
    for (origin, project) in placements.keys() {
        if !expected_projects.contains(&(*origin, *project)) {
            drifts.push(format!(
                "{project} ({}) : placed, missing from lock",
                origin.as_str()
            ));
        }
    }

    drifts
}

/// A dependency a mod declares toward a specific build.
pub(super) struct Requirement<'a> {
    /// The mod that requires it, as it's named on screen.
    pub by: &'a str,
    pub origin: Origin,
    pub project: &'a str,
    pub build: &'a str,
}

/// The pinned dependencies an author declares that the pack does not honor.
///
/// A mod can require **a specific build** of another, not a range: that's
/// the case with Iris, whose compatibility mixins target an exact Sodium
/// version. The launcher does turn this requirement into a pinned request —
/// but an explicit request from the manifest overrides a declared
/// dependency, and that's intentional: the manifest is sovereign.
///
/// What isn't intentional is silence. A manifest that requests "sodium"
/// without a version gets the latest build, Iris gets its mixins applied to
/// a class that no longer exists, and the game fails on first connect with
/// a `ClassNotFoundException` that no one links back to the pack. Saying so
/// here costs one comparison and saves an evening.
pub(super) fn unsatisfied_dependencies<'a>(
    placements: &[Placement<'a>],
    requirements: impl Iterator<Item = Requirement<'a>>,
) -> Vec<String> {
    let placements: std::collections::BTreeMap<(Origin, &str), &str> = placements
        .iter()
        .map(|(origin, project, build)| ((*origin, *project), *build))
        .collect();

    requirements
        .filter_map(|requirement| {
            let placed = placements.get(&(requirement.origin, requirement.project))?;
            // The dependency is missing from the pack: that's the job of
            // `Plan::unresolved`, not ours. Here we only talk about builds
            // that are present but differ from what's required.
            (*placed != requirement.build).then(|| {
                format!(
                    "{} requires build {} of {}, but {placed} is installed",
                    requirement.by, requirement.build, requirement.project
                )
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "conformance.test.rs"]
mod tests;
