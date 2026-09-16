//! Poser le pack sur cette machine.

use anyhow::Result;
use mc_pack::source::Source;

use super::verify::report_unresolved;

pub async fn install(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let outcome = mc_pack::install(source, options, &|line| println!("{line}")).await?;

    println!("\nInstance « {} »", outcome.instance.name);
    println!("  pack     : {}", outcome.source);
    if outcome.from_cache {
        println!("             (hors-ligne — copie locale, pas le pack publié)");
    }
    println!("  jeu      : {}", outcome.instance.game_dir.display());
    println!(
        "  mods     : {} côté client, {} côté serveur",
        outcome.client_mods, outcome.server_mods
    );
    println!("  serveur  : {}", outcome.server_dir.display());
    println!("  verrou   : {}", outcome.lock_path.display());

    if !outcome.removed.is_empty() {
        println!("  retirés  : {}", outcome.removed.join(", "));
    }
    if let Some(previous) = &outcome.previous_lock {
        let changes = outcome.lock.diff(previous);
        if !changes.is_empty() {
            println!("\nChangements depuis le verrou précédent :");
            for line in changes {
                println!("  {line}");
            }
        }
    }
    report_unresolved(&outcome.lock);
    Ok(())
}
