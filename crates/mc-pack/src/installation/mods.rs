//! Resolve the mods, then place them in the instance.

mod deployment;

use std::path::Path;

use anyhow::Result;

use std::sync::Arc;

use crate::Options;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::progress::Report;

/// What the "mods" step leaves behind.
pub(super) struct Placement {
    pub plan: mc_mods::Plan,
    pub instance: mc_instance::Instance,
    pub server_dir: std::path::PathBuf,
    pub client_mods: usize,
    pub server_mods: usize,
    pub removed: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn place(
    manifest: &Manifest,
    previous: Option<&Lockfile>,
    options: &Options,
    replay: bool,
    java: &Path,
    neoforge_version: &str,
    dl: &mc_dl::Downloader,
    report: &Arc<dyn Report>,
) -> Result<Placement> {
    let layout = &options.layout;
    // The registry sets up its own HTTP client: without this observer, mods
    // would be the only step to run silently, and it's the longest one
    // after assets.
    let registry =
        mc_mods::Registry::observed(layout.cache().join("mods"), super::observer(report))?
            // And it also reports where the RESOLUTION stands: that's the
            // part that takes time, and it barely reports anything.
            .announcing(announcer(report));
    let requests = if replay {
        let lock = previous.expect("checked above");
        tracing::info!(
            builds = lock.mods.len(),
            "Lock replay: {} pinned builds",
            lock.mods.len()
        );
        report.note(&format!(
            "Mods: {} builds replayed from the lock",
            lock.mods.len()
        ));
        lock.requests()
    } else {
        tracing::info!(
            requested = manifest.mods.len(),
            "Resolving {} requested mods",
            manifest.mods.len()
        );
        report.note("Resolving mods…");
        manifest.requests()?
    };

    let plan = mc_mods::resolve(&registry, &requests, &manifest.minecraft, "neoforge").await?;
    let added = added_by_dependency(plan.mods.iter().map(|m| &m.reason));
    tracing::info!(
        total = plan.mods.len(),
        added,
        unresolved = plan.unresolved.len(),
        "{} mods resolved, {added} added by dependency",
        plan.mods.len()
    );
    report.note(&format!(
        "  {} mods, {added} added by dependency resolution",
        plan.mods.len()
    ));
    for entry in plan
        .mods
        .iter()
        .filter(|m| matches!(m.reason, mc_mods::Reason::Implicit { .. }))
    {
        report.note(&format!(
            "  · {} — {}",
            entry.candidate.slug,
            entry.reason.describe()
        ));
    }
    deployment::deploy(
        plan,
        options,
        manifest,
        java,
        neoforge_version,
        dl,
        report.as_ref(),
    )
    .await
}

/// How many mods the pack gained without the manifest requesting them.
///
/// This is the number that explains how a thirty-line manifest installs a
/// hundred mods: the rest comes from dependencies. Counting the others —
/// the ones the manifest names — would say the opposite, and suggest a
/// resolution that found nothing.
fn added_by_dependency<'a>(reasons: impl Iterator<Item = &'a mc_mods::Reason>) -> usize {
    reasons
        .filter(|reason| **reason != mc_mods::Reason::Explicit)
        .count()
}

#[cfg(test)]
#[path = "mods.test.rs"]
mod tests;

/// The thread that carries resolution progress to the report.
///
/// Twin of `super::observer`, for the other half of the time spent in the
/// Mods step: the part where APIs are queried rather than bytes downloaded.
fn announcer(report: &Arc<dyn Report>) -> mc_mods::Progress {
    let report = Arc::clone(report);
    Arc::new(move |done, total| report.resolution(done, total))
}
