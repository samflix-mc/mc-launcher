//! Installe un pack décrit par un manifeste JSON.
//!
//!     mc-pack install packs/samflix.json
//!     mc-pack install packs/samflix.json --locked      rejoue le verrou
//!     mc-pack install packs/samflix.json --with-server installe aussi le serveur
//!     mc-pack lock    packs/samflix.json               résout sans installer le jeu
//!     mc-pack verify  packs/samflix.json [--deep]
//!     mc-pack diagnostic                               journaux et télémétrie
//!
//! Options communes :
//!     --instance <NOM>   nom de l'instance, par défaut celui du pack
//!     --data <DIR>       racine des données du launcher
//!
//! `RUST_LOG` règle la verbosité de la console ; le fichier de journal garde le
//! détail quoi qu'il arrive.

use anyhow::{Result, bail};
use mc_pack::lockfile::{LockedLoader, Lockfile};
use mc_pack::manifest::Manifest;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    // Avant tout le reste : une erreur de lecture d'arguments mérite déjà
    // d'être journalisée, et le guard doit vivre jusqu'à la fin du programme
    // pour que le journal et les incidents partent complètement.
    let _log = mc_log::init("mc-pack");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().cloned() else {
        usage();
        std::process::exit(2);
    };

    let mut manifest_path: Option<PathBuf> = None;
    let mut options = mc_pack::Options::default();
    let mut deep = false;

    let mut incident_test = false;

    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--incident-test" => incident_test = true,
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

    // Le diagnostic n'a pas besoin de manifeste : c'est justement ce qu'on
    // lance quand on ne sait pas encore ce qui va de travers.
    if command == "diagnostic" {
        return diagnostic(&_log, incident_test);
    }

    let Some(manifest_path) = manifest_path else {
        usage();
        std::process::exit(2);
    };

    tracing::info!(commande = %command, manifeste = %manifest_path.display(), "démarrage");

    let result = match command.as_str() {
        "install" => install(&manifest_path, &options).await,
        "lock" => lock(&manifest_path, &options).await,
        "verify" => verify(&manifest_path, &options, deep),
        other => {
            eprintln!("commande inconnue : {other}");
            usage();
            std::process::exit(2);
        }
    };

    // Une erreur remontée jusqu'ici met fin au programme : c'est le dernier
    // endroit où elle peut devenir un incident plutôt qu'un simple message.
    if let Err(error) = &result {
        tracing::error!(commande = %command, erreur = ?error, "échec");
        if let Some(path) = _log.log_path() {
            eprintln!("\nJournal détaillé : {}", path.display());
        }
    }
    result
}

fn usage() {
    eprintln!(
        "usage :\n  \
         mc-pack install <manifeste> [--locked] [--with-server] [--instance NOM] [--data DIR]\n  \
         mc-pack lock    <manifeste> [--data DIR]\n  \
         mc-pack verify  <manifeste> [--deep] [--data DIR]\n  \
         mc-pack diagnostic [--incident-test]"
    );
}

/// Montre où vont les journaux et si la remontée d'incidents est active.
///
/// Première chose à demander à quelqu'un dont l'installation échoue : la
/// réponse tient en dix lignes et dit où trouver le reste.
fn diagnostic(log: &mc_log::Guard, incident_test: bool) -> Result<()> {
    println!("Journaux");
    match log.log_path() {
        Some(path) => println!("  fichier    : {}", path.display()),
        None => println!("  fichier    : indisponible (répertoire non inscriptible)"),
    }
    println!("  répertoire : {}", mc_log::log_dir().display());
    println!(
        "  console    : {}",
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info (régler RUST_LOG)".into())
    );

    println!("\nRemontée d'incidents");
    println!(
        "  état       : {}",
        if mc_log::telemetry_active() {
            "active — couper avec SAMFLIX_TELEMETRY=0"
        } else {
            "coupée"
        }
    );
    println!(
        "  version    : {}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("inconnue")
    );

    if incident_test {
        if !mc_log::telemetry_active() {
            println!("\nRien à envoyer : la remontée est coupée.");
            return Ok(());
        }
        println!("\nEnvoi d'un incident de test…");
        let (id, envoye) = mc_log::send_test_event();
        println!("  identifiant : {id}");
        if envoye {
            println!("  envoi       : abouti — à retrouver dans Sentry sous cet identifiant");
            println!("  canaux      : incident (Issues) et journal structuré (Logs)");
            println!("\n  Dans l'onglet Logs, l'entrée « ligne de journal de test » porte deux");
            println!("  attributs, et c'est leur différence qui vaut vérification :");
            println!("    avec_mot_cle → [Filtered] : Sentry filtre lui-même, ne prouve rien");
            println!("    jeton_nu     → [secret]   : c'est notre censure qui a agi");
            println!("  Un jeton lisible en face de « jeton_nu » signalerait une fuite.");
        } else {
            println!("  envoi       : ÉCHOUÉ (file non vidée avant expiration)");
            println!("                réseau bloqué, DSN erroné ou projet inexistant");
            std::process::exit(1);
        }
    }
    Ok(())
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
