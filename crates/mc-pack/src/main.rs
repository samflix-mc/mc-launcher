//! Installe un pack décrit par un manifeste JSON.
//!
//!     mc-pack install packs/samflix.json
//!     mc-pack install packs/samflix.json --locked      rejoue le verrou
//!     mc-pack install packs/samflix.json --with-server installe aussi le serveur
//!     mc-pack lock    packs/samflix.json               résout sans installer le jeu
//!     mc-pack verify  packs/samflix.json [--deep]
//!
//! Options communes :
//!     --instance <NOM>   nom de l'instance, par défaut celui du pack
//!     --data <DIR>       racine des données du launcher

use anyhow::{Result, bail};
use mc_pack::lockfile::{LockedLoader, Lockfile};
use mc_pack::manifest::Manifest;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().cloned() else {
        usage();
        std::process::exit(2);
    };

    let mut manifest_path: Option<PathBuf> = None;
    let mut options = mc_pack::Options::default();
    let mut deep = false;

    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--locked" => options.locked = true,
            "--with-server" => options.with_server = true,
            "--deep" => deep = true,
            "--instance" => options.instance_name = rest.next().cloned(),
            "--data" => {
                let Some(dir) = rest.next() else {
                    bail!("--data attend un répertoire");
                };
                options.layout = mc_instance::Layout::new(PathBuf::from(dir));
            }
            other if other.starts_with("--") => bail!("option inconnue : {other}"),
            path => manifest_path = Some(PathBuf::from(path)),
        }
    }

    let Some(manifest_path) = manifest_path else {
        usage();
        std::process::exit(2);
    };

    match command.as_str() {
        "install" => install(&manifest_path, &options).await,
        "lock" => lock(&manifest_path, &options).await,
        "verify" => verify(&manifest_path, &options, deep),
        other => {
            eprintln!("commande inconnue : {other}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    eprintln!(
        "usage :\n  \
         mc-pack install <manifeste> [--locked] [--with-server] [--instance NOM] [--data DIR]\n  \
         mc-pack lock    <manifeste> [--data DIR]\n  \
         mc-pack verify  <manifeste> [--deep] [--data DIR]"
    );
}

async fn install(manifest_path: &Path, options: &mc_pack::Options) -> Result<()> {
    let outcome = mc_pack::install(manifest_path, options, &|line| println!("{line}")).await?;

    println!("\nInstance « {} »", outcome.instance.name);
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

/// Résout et écrit le verrou sans installer le jeu.
///
/// C'est ce qu'on lance en revue : le verrou montre les versions retenues et
/// les dépendances ajoutées, sans attendre le téléchargement de huit cents
/// mégaoctets d'assets.
async fn lock(manifest_path: &Path, options: &mc_pack::Options) -> Result<()> {
    let manifest = Manifest::load(manifest_path)?;
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    let neoforge_version = if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, &dl).await?
    } else {
        manifest.loader.version.clone()
    };

    let registry = mc_mods::Registry::new(options.layout.cache().join("mods"))?;
    let plan = mc_mods::resolve(
        &registry,
        &manifest.requests()?,
        &manifest.minecraft,
        "neoforge",
    )
    .await?;

    let lock_path = Lockfile::path_for(manifest_path);
    let previous = lock_path
        .is_file()
        .then(|| Lockfile::load(&lock_path))
        .transpose()?;
    let lock = Lockfile::from_plan(
        &manifest.name,
        &manifest.minecraft,
        LockedLoader {
            kind: manifest.loader.kind.clone(),
            version: neoforge_version,
        },
        manifest.java.unwrap_or(21),
        &plan,
    );
    lock.save(&lock_path)?;

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
        let changes = lock.diff(&previous);
        if !changes.is_empty() {
            println!("\nChangements :");
            for line in changes {
                println!("  {line}");
            }
        }
    }
    println!("\nVerrou écrit dans {}", lock_path.display());
    report_unresolved(&lock);
    Ok(())
}

fn verify(manifest_path: &Path, options: &mc_pack::Options, deep: bool) -> Result<()> {
    let problems = mc_pack::verify(manifest_path, options, deep)?;
    if problems.is_empty() {
        println!("Installation complète et conforme au verrou.");
        return Ok(());
    }
    eprintln!("{} anomalies :", problems.len());
    for problem in &problems {
        eprintln!("  {problem}");
    }
    std::process::exit(1);
}

/// Une dépendance introuvable n'empêche pas d'installer, mais empêchera le jeu
/// de démarrer : elle est signalée là où on la verra.
fn report_unresolved(lock: &Lockfile) {
    if lock.unresolved.is_empty() {
        return;
    }
    eprintln!("\nDépendances introuvables :");
    for missing in &lock.unresolved {
        eprintln!(
            "  {} — exigé par {} ({})",
            missing.mod_id, missing.required_by, missing.side
        );
    }
    eprintln!(
        "  Ces mods manquent sur Modrinth comme sur CurseForge : le jeu refusera de démarrer."
    );
}
