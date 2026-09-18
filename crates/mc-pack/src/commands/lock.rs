//! Resolve the pack and freeze the result into a lock.

mod preparation;
mod report;
mod resolution;

use anyhow::Result;
use mc_pack::lockfile::{LockedLoader, Lockfile};
use mc_pack::source::Source;

/// Resolves and writes the lock without installing the game.
///
/// This is what you run for a review: the lock shows the versions picked
/// and the dependencies added, without waiting on eight hundred megabytes
/// of asset downloads.
/// Out of scope for mutation testing: this command resolves the pack
/// online, writes the lock, and announces it. Each of these three parts is
/// tested separately — resolution over at mc-mods, the lock write over at
/// `Lockfile`, the report by its own suite.
#[mutants::skip]
pub async fn lock(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let (manifest_path, manifest) = preparation::manifest_to_lock(source)?;
    let (plan, neoforge_version) = resolution::resolve(&manifest, options).await?;

    let lock_path = Lockfile::path_for(&manifest_path);
    let previous = lock_path
        .is_file()
        .then(|| Lockfile::load(&lock_path))
        .transpose()?;
    let lock = Lockfile::from_plan(
        &manifest,
        LockedLoader {
            kind: manifest.loader.kind.clone(),
            version: neoforge_version,
        },
        // Not `manifest.java.unwrap_or(21)`. The lock is what's authoritative
        // for installation AND for checking Java on every launch: writing a
        // constant here would freeze a guessed number where installation, on
        // the other hand, asked Mojang. The two could diverge, and it's the
        // lock that would be wrong.
        //
        // The exact same expression as at step 4 of installation, word for
        // word — including the call to `Manifest::java_major`, which carries
        // the manifest's priority rule. Replacing it with `java_required` alone
        // would make `lock` lose the ability to impose a major version,
        // which `manifest/reading.requests.test.rs` locks in.
        manifest.java_major(
            mc_instance::vanilla::java_required(
                &manifest.minecraft,
                &mc_dl::Downloader::new(mc_dl::USER_AGENT)?,
            )
            .await?,
        ),
        &plan,
    );
    lock.save(&lock_path)?;

    // The lock is the command's product: its position and what changed are
    // what you'd look for in the log if a review turns up something odd.
    let changes = previous.as_ref().map(|p| lock.diff(p).len()).unwrap_or(0);
    tracing::info!(
        lock = %lock_path.display(),
        changes,
        new = previous.is_none(),
        "Lock written to {} ({})",
        lock_path.display(),
        match (previous.is_none(), changes) {
            (true, _) => "new".to_string(),
            (false, 0) => "unchanged".to_string(),
            (false, n) => format!("{n} changes"),
        }
    );
    report::announce(&lock, &lock_path, previous.as_ref());
    Ok(())
}
