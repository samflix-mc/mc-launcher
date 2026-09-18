//! Install a pack: seven steps, in this order and no other.
//!
//! Seven, like `Step::ALL` — see the header of `lib.rs`, which names them.

mod conformance;
mod java;
mod loader;
mod lock;
mod mods;
mod mojang;
mod report;

use std::sync::Arc;

use anyhow::Result;

use crate::progress::{Report, Step};
use crate::source::{Pack, Source};
use crate::{Options, Outcome};

#[tracing::instrument(
    name = "installation",
    skip(options, report),
    fields(pack, minecraft, source = %source.describe(), replay, server = options.with_server)
)]
pub async fn install(
    source: &Source,
    options: &Options,
    report: Arc<dyn Report>,
) -> Result<Outcome> {
    // The HTTP client carries the observer: everything that follows — the
    // pack, the game files, the NeoForge installer — goes through it and
    // reports itself without every step having to handle it.
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?.observe(observer(&report));

    // --- 1. The pack ----------------------------------------------------------
    report.step(Step::Pack);
    let Pack {
        manifest,
        lock: previous_lock,
        lock_path,
        replay,
        from_cache,
    } = source.load(&dl).await?;

    let replay = should_replay(replay, options.locked);

    // Filled in once the manifest is read: the span carries them, so
    // everything that follows is attached to the pack without repeating it on
    // every line.
    tracing::Span::current().record("pack", &manifest.name);
    tracing::Span::current().record("minecraft", &manifest.minecraft);
    tracing::Span::current().record("replay", replay);

    if from_cache {
        report.note("Offline: pack resumed from the last known copy.");
    }

    let layout = &options.layout;
    let shared = layout.shared();

    // --- 1 bis. The purge, if any ---------------------------------------
    //
    // Here and nowhere else: the manifest has been read — so the requested
    // generation is known — and nothing has been written to the instance yet.
    //
    // The instance is NOT in scope at this point: it is only built at step 6.
    // It is therefore derived here with EXACTLY the same expression, or risk
    // purging a different directory than the one that will be filled. The
    // name is safe: `manifest/control.rs:37-44` already rejects a `name` that
    // would escape the root, and it's that check that allows a recursive
    // delete on a path that came from the network.
    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let placed_state = crate::state::LocalState::read(&crate::state::path(&instance));

    // The generation comes from the MANIFEST, never from the lock. At this
    // point, `previous_lock` is the *published* lock being replayed, but the
    // *previous* lock on a local manifest being re-resolved: the two don't
    // answer the same question.
    let purge = match crate::state::decide(placed_state.as_ref(), manifest.generation) {
        crate::state::Before::Differential => crate::state::Purge::default(),
        crate::state::Before::Purge => {
            // The note says "in progress", the report will say what was
            // actually done: `note` is a single slot that seven later calls
            // will overwrite within the second.
            report.note("Full reinstall requested by the pack…");
            tracing::info!(
                placed_generation = placed_state.as_ref().map(|e| e.generation),
                requested_generation = manifest.generation,
                "purge before installation"
            );
            crate::state::purge(&instance.game_dir)
        }
    };

    // --- 2. The loader ------------------------------------------------------
    report.step(Step::Loader);
    let neoforge_version =
        loader::version(&manifest, previous_lock.as_ref(), &lock_path, replay, &dl).await?;
    report.note(&format!(
        "Minecraft {} — NeoForge {neoforge_version}",
        manifest.minecraft
    ));

    // --- 3. Mojang files ----------------------------------------------
    report.step(Step::Minecraft);
    let game = mojang::place(&manifest.minecraft, &shared, &dl, report.as_ref()).await?;

    // --- 4. Java -------------------------------------------------------------
    report.step(Step::Java);
    let java_major = manifest.java_major(Some(game.java_major));
    let java = java::runtime(java_major, layout, &report).await?;
    report.note(&format!(
        "Java {} — {}",
        java.version.full,
        java.path.display()
    ));

    // --- 5. NeoForge ---------------------------------------------------------
    report.step(Step::NeoForge);
    report.note("NeoForge loader…");
    loader::place(&neoforge_version, &shared, layout, &java.path, &dl).await?;

    // --- 6. Mods -------------------------------------------------------------
    report.step(Step::Mods);
    let placement = mods::place(
        &manifest,
        previous_lock.as_ref(),
        options,
        replay,
        &java.path,
        &neoforge_version,
        &dl,
        &report,
    )
    .await?;

    // --- 7. The lock --------------------------------------------------------
    report.step(Step::Lock);
    let lock = lock::retain(
        &manifest,
        &neoforge_version,
        java_major,
        &placement.plan,
        previous_lock.as_ref(),
        &lock_path,
        replay,
    )?;

    // Replaying a published lock means obeying it — but it still has to be
    // checked that it was actually achieved. A build withdrawn from its
    // source, a deduplication that resolved differently: the installation
    // finishes "fine", and the drift only shows up at connect time, as an
    // ejection that never names its cause.
    let placements: Vec<conformance::Placement<'_>> = placement
        .plan
        .mods
        .iter()
        .map(|m| {
            (
                m.candidate.origin,
                m.candidate.project_id.as_str(),
                m.candidate.version_id.as_str(),
            )
        })
        .collect();

    let mut drifts = Vec::new();

    if replay {
        drifts.extend(conformance::drifts(&lock, placements.iter().copied()));
    }

    // An author can require a specific build of another mod — Iris's
    // compatibility mixins target an exact Sodium version. An explicit
    // request from the manifest overrides this requirement, and that's
    // intentional; staying silent about it isn't, because the game then
    // fails on first connect with an error that never names the pack.
    drifts.extend(conformance::unsatisfied_dependencies(
        &placements,
        placement.plan.mods.iter().flat_map(|m| {
            m.candidate.declared_deps.iter().filter_map(move |dep| {
                Some(conformance::Requirement {
                    by: m.candidate.slug.as_str(),
                    origin: m.candidate.origin,
                    project: dep.project_id.as_str(),
                    build: dep.version_id.as_deref()?,
                })
            })
        }),
    ));

    for drift in &drifts {
        tracing::warn!(drift, "installation diverges from what was expected");
        report.note(&format!("  ⚠ {drift}"));
    }

    // --- 8. Retain what was just placed ----------------------------------
    //
    // After everything else: a state written before the end would describe
    // an installation that didn't finish, and the next launch would believe
    // there was nothing to do. A write failure does not fail the
    // installation — it succeeded — but it costs a purge on the next launch,
    // and that's annoying enough to be logged.
    match lock.digest() {
        Ok(digest) => {
            let state = crate::state::LocalState::new(
                digest,
                manifest.generation,
                crate::lockfile::now_utc(),
            );
            if let Err(error) = state.write(&crate::state::path(&instance)) {
                tracing::warn!(
                    error = %error,
                    "local state not written: the next launch will go through a purge"
                );
            }
        }
        Err(error) => tracing::warn!(error = %error, "lock digest could not be computed"),
    }

    Ok(report::assemble(
        source,
        placement,
        java,
        game,
        lock,
        lock_path,
        previous_lock,
        neoforge_version,
        from_cache,
        drifts,
        purge,
    ))
}

/// The thread that connects downloads to the report.
///
/// `mc-dl` only knows closures, `mc-pack` only knows a report: this is the
/// solder joint, and the only place in the crate where the two see each
/// other.
pub(crate) fn observer(report: &Arc<dyn Report>) -> mc_dl::Observer {
    let report = Arc::clone(report);
    Arc::new(move |progress| report.download(progress))
}

/// Should the lock be replayed rather than resolved again?
///
/// Two reasons, independent of each other. A remote pack always replays:
/// it's the published lock that decides the versions, not the player's
/// machine. And `--locked` requires it explicitly, even on a local manifest
/// currently being edited. Confusing the two would resolve a published pack
/// again, and the player would not get the versions the network validated.
fn should_replay(remote_pack: bool, locked: bool) -> bool {
    remote_pack || locked
}

#[cfg(test)]
#[path = "installation.test.rs"]
mod tests;
