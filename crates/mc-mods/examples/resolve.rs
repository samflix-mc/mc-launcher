//! Resolves a list of mods and shows where each jar comes from.
//!
//!     cargo run -p mc-mods --example resolve -- jei jade modernfix
//!
//! Used to manually check what resolution adds on its own: the "implicit
//! dependency" lines are the ones no API announced.

use anyhow::Result;
use mc_mods::{Reason, Registry, Request};

#[tokio::main]
async fn main() -> Result<()> {
    let mut slugs: Vec<String> = std::env::args().skip(1).collect();
    // `--detail` shows what each jar truly declares: it's this reading, not
    // the APIs, that decides which dependencies to catch up on.
    let detail = slugs.iter().any(|a| a == "--detail");
    // `--jars-only` ignores what the APIs declare: everything that shows up
    // beyond the requested mods then comes from reading the jars.
    let jars_only = slugs.iter().any(|a| a == "--jars-only");
    // `--curseforge` bypasses Modrinth, to exercise the other source.
    let force_cf = slugs.iter().any(|a| a == "--curseforge");
    slugs.retain(|a| a != "--detail" && a != "--jars-only" && a != "--curseforge");
    if slugs.is_empty() {
        eprintln!("usage: resolve [--detail] [--jars-only] [--curseforge] <slug> [<slug>…]");
        std::process::exit(2);
    }

    let cache = mc_paths::current().data.join("cache").join("mods");
    let registry = Registry::new(cache)?;
    let requests: Vec<Request> = slugs
        .into_iter()
        .map(|slug| {
            let mut request = Request::new(slug);
            if force_cf {
                request.source = Some(mc_mods::Origin::CurseForge);
            }
            request
        })
        .collect();

    let options = mc_mods::Options {
        follow_declared: !jars_only,
    };
    let plan = mc_mods::resolve_with(&registry, &requests, "1.21.1", "neoforge", options).await?;

    println!("{} mods", plan.mods.len());
    for entry in &plan.mods {
        println!(
            "  {:<22} {:<38} {:<7} {:<11} {}",
            entry.candidate.slug,
            entry.candidate.file_name,
            entry.side.as_str(),
            entry.candidate.origin.as_str(),
            match &entry.reason {
                Reason::Explicit => String::new(),
                other => other.describe(),
            }
        );
        if detail {
            println!("      provides: {:?}", entry.provides);
            println!(
                "      requires: {:?}",
                entry
                    .requires
                    .iter()
                    .map(|r| format!("{} {}", r.mod_id, r.side.as_str()))
                    .collect::<Vec<_>>()
            );
        }
    }
    for missing in &plan.unresolved {
        println!(
            "  MISSING {} — required by {}",
            missing.mod_id, missing.required_by
        );
    }
    Ok(())
}
