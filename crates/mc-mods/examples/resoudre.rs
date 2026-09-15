//! Résout une liste de mods et montre d'où vient chaque jar.
//!
//!     cargo run -p mc-mods --example resoudre -- jei jade modernfix
//!
//! Sert à vérifier à la main ce que la résolution ajoute d'elle-même : les
//! lignes « dépendance implicite » sont celles qu'aucune API n'annonçait.

use anyhow::Result;
use mc_mods::{Reason, Registry, Request};

#[tokio::main]
async fn main() -> Result<()> {
    let mut slugs: Vec<String> = std::env::args().skip(1).collect();
    // `--detail` montre ce que chaque jar déclare vraiment : c'est cette
    // lecture, et non les API, qui décide des dépendances à rattraper.
    let detail = slugs.iter().any(|a| a == "--detail");
    // `--jars-seuls` ignore ce que les API déclarent : tout ce qui apparaît en
    // plus des mods demandés vient alors de la lecture des jars.
    let jars_only = slugs.iter().any(|a| a == "--jars-seuls");
    // `--curseforge` court-circuite Modrinth, pour éprouver l'autre source.
    let force_cf = slugs.iter().any(|a| a == "--curseforge");
    slugs.retain(|a| a != "--detail" && a != "--jars-seuls" && a != "--curseforge");
    if slugs.is_empty() {
        eprintln!("usage : resoudre [--detail] [--jars-seuls] [--curseforge] <slug> [<slug>…]");
        std::process::exit(2);
    }

    let cache = mc_dl::data_dir().join("cache").join("mods");
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
            println!("      fournit : {:?}", entry.provides);
            println!(
                "      exige   : {:?}",
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
            "  MANQUE {} — exigé par {}",
            missing.mod_id, missing.required_by
        );
    }
    Ok(())
}
