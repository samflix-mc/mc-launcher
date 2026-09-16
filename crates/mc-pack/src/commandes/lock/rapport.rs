//! Ce que la commande affiche du verrou qu'elle vient d'écrire.

use std::path::Path;

use mc_pack::lockfile::Lockfile;

use super::super::verify::report_unresolved;

pub(super) fn annoncer(lock: &Lockfile, lock_path: &Path, previous: Option<&Lockfile>) {
    println!(
        "NeoForge {} — {} mods",
        lock.loader.version,
        lock.mods.len()
    );
    for entry in &lock.mods {
        println!(
            "  {:<24} {:<40} {:<7} {}",
            entry.slug, entry.file_name, entry.side, entry.reason
        );
    }
    if let Some(previous) = previous {
        let changes = lock.diff(previous);
        if !changes.is_empty() {
            println!("\nChangements :");
            for line in changes {
                println!("  {line}");
            }
        }
    }
    println!("\nVerrou écrit dans {}", lock_path.display());
    report_unresolved(lock);
}
