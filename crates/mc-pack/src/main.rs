//! Installe un pack décrit par un manifeste JSON.
//!
//!     mc-pack install packs/samflix.json
//!     mc-pack install packs/samflix.json --locked      rejoue le verrou
//!     mc-pack install packs/samflix.json --with-server installe aussi le serveur
//!     mc-pack lock    packs/samflix.json               résout sans installer le jeu
//!     mc-pack verify  packs/samflix.json [--deep]
//!     mc-pack launch  packs/samflix.json --pseudo Sam --serveur mc.exemple.fr
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
    let mut pseudo: Option<String> = None;
    let mut serveur: Option<String> = None;
    let mut memoire: Option<u32> = None;
    let mut afficher = false;

    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--incident-test" => incident_test = true,
            "--locked" => options.locked = true,
            "--with-server" => options.with_server = true,
            "--deep" => deep = true,
            "--pseudo" => pseudo = rest.next().cloned(),
            "--serveur" => serveur = rest.next().cloned(),
            "--memoire" => memoire = rest.next().and_then(|v| v.parse().ok()),
            "--afficher" => afficher = true,
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

    // Span racine : tout ce qui suit lui est rattaché, et sa durée est celle de
    // la commande. Dans le fichier comme dans Sentry, une exécution se lit ainsi
    // d'un bloc même quand plusieurs se succèdent.
    let span = tracing::info_span!(
        "commande",
        nom = %command,
        manifeste = %manifest_path.display(),
        environnement = mc_log::environment::current().as_str(),
    );
    let _entree = span.enter();

    let debut = std::time::Instant::now();
    tracing::info!(
        environnement = mc_log::environment::current().as_str(),
        "mc-pack {command} sur {} — environnement {}",
        manifest_path.display(),
        mc_log::environment::current().as_str()
    );

    let result = match command.as_str() {
        "install" => install(&manifest_path, &options).await,
        "launch" => launch(&manifest_path, &options, pseudo, serveur, memoire, afficher).await,
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
    match &result {
        Ok(_) => tracing::info!(
            duree_ms = debut.elapsed().as_millis(),
            "mc-pack {command} terminé en {:.1} s",
            debut.elapsed().as_secs_f64()
        ),
        Err(error) => {
            tracing::error!(
                duree_ms = debut.elapsed().as_millis(),
                erreur = ?error,
                "mc-pack {command} a échoué après {:.1} s : {error}",
                debut.elapsed().as_secs_f64()
            );
            if let Some(path) = _log.log_path() {
                eprintln!("\nJournal détaillé : {}", path.display());
            }
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
         mc-pack launch  <manifeste> --pseudo NOM [--serveur HOTE[:PORT]] [--memoire MO] [--afficher]\n  \
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
    // Affiché avec sa provenance : un environnement inattendu se remonte ainsi
    // à sa source sans avoir à relire le code.
    println!(
        "  environnement : {} ({})",
        mc_log::environment::current().as_str(),
        mc_log::environment::origin()
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
            println!("\n  Dans l'onglet Logs, sur « ligne de journal de test », un seul");
            println!("  attribut tranche — les deux autres sont filtrés par Sentry lui-même");
            println!("  et ne diraient rien de notre censure :");
            println!("    temoin_chemin → « ~/… »  : before_send_log a tourné");
            println!("                  → « /home/… » : il n'a pas tourné, il y a une fuite");
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
    tracing::info!(
        pack = %manifest.name,
        minecraft = %manifest.minecraft,
        demandes = manifest.mods.len(),
        "Pack « {} » : Minecraft {}, {} mods demandés",
        manifest.name,
        manifest.minecraft,
        manifest.mods.len()
    );

    let neoforge_version = if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, &dl).await?
    } else {
        manifest.loader.version.clone()
    };
    tracing::info!(
        neoforge = %neoforge_version,
        epingle = !manifest.loader.is_latest(),
        "NeoForge {} retenu ({})",
        neoforge_version,
        if manifest.loader.is_latest() {
            "dernière version publiée"
        } else {
            "épinglé par le manifeste"
        }
    );

    let registry = mc_mods::Registry::new(options.layout.cache().join("mods"))?;
    let plan = mc_mods::resolve(
        &registry,
        &manifest.requests()?,
        &manifest.minecraft,
        "neoforge",
    )
    .await?;

    let ajoutes = plan
        .mods
        .iter()
        .filter(|m| m.reason != mc_mods::Reason::Explicit)
        .count();
    tracing::info!(
        total = plan.mods.len(),
        ajoutes,
        non_resolus = plan.unresolved.len(),
        "{} mods résolus, dont {ajoutes} ajoutés par dépendance",
        plan.mods.len()
    );

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

    // Le verrou est le produit de la commande : sa position et ce qui a bougé
    // sont ce qu'on cherchera dans le journal si une revue surprend.
    let changements = previous.as_ref().map(|p| lock.diff(p).len()).unwrap_or(0);
    tracing::info!(
        verrou = %lock_path.display(),
        changements,
        nouveau = previous.is_none(),
        "Verrou écrit dans {} ({})",
        lock_path.display(),
        match (previous.is_none(), changements) {
            (true, _) => "nouveau".to_string(),
            (false, 0) => "inchangé".to_string(),
            (false, n) => format!("{n} changements"),
        }
    );

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
        tracing::info!(
            exhaustif = deep,
            "Installation conforme au verrou{}",
            if deep { ", empreintes comprises" } else { "" }
        );
        println!("Installation complète et conforme au verrou.");
        return Ok(());
    }
    // Une anomalie de vérification n'est pas une panne du programme : elle
    // décrit l'installation. D'où `warn` plutôt que `error` — l'incident, c'est
    // le code de sortie, que l'appelant voit déjà.
    tracing::warn!(
        anomalies = problems.len(),
        exhaustif = deep,
        "Installation non conforme : {} anomalies",
        problems.len()
    );
    for problem in &problems {
        tracing::debug!(anomalie = %problem, "détail");
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

/// Démarre le jeu sur l'instance d'un pack.
///
/// L'installation n'est pas relancée : `launch` suppose qu'elle a eu lieu et le
/// dit clairement si un fichier manque. Installer et jouer sont deux gestes
/// distincts, et les enchaîner ferait attendre huit cents mégaoctets à qui
/// voulait seulement lancer une partie.
async fn launch(
    manifest_path: &Path,
    options: &mc_pack::Options,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
    afficher: bool,
) -> Result<()> {
    let Some(pseudo) = pseudo else {
        bail!("launch attend --pseudo <NOM>");
    };

    let manifest = Manifest::load(manifest_path)?;
    let lock_path = Lockfile::path_for(manifest_path);
    let lock = Lockfile::load(&lock_path).map_err(|_| {
        anyhow::anyhow!(
            "{} absent : lancer « mc-pack install » avant de jouer",
            lock_path.display()
        )
    })?;

    let layout = &options.layout;
    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let version_id = mc_instance::neoforge::version_id(&lock.loader.version);

    // Le Java du verrou, pas celui du système : c'est avec lui que NeoForge a
    // été installé.
    let java = mc_java::ensure(lock.java, &layout.runtime()).await?;

    // Hors ligne tant que l'application Azure n'est pas approuvée. L'UUID suit
    // la règle du serveur vanilla, donc le joueur garde le même d'une session à
    // l'autre — inventaire et permissions compris.
    let offline = mc_auth::offline_session(&pseudo);
    let session = mc_instance::launch::Session::offline(&offline.profile.name, &offline.profile.id);

    let launch_options = mc_instance::launch::LaunchOptions {
        memory_mb: memoire,
        quick_play: serveur.map(mc_instance::launch::QuickPlay::Multiplayer),
        ..Default::default()
    };

    let command = mc_instance::launch::build(
        &version_id,
        &layout.shared(),
        &instance.game_dir,
        &java.path,
        &session,
        &launch_options,
    )?;

    println!("Instance « {} »", instance.name);
    println!("  version : {version_id}");
    println!("  joueur  : {} ({})", session.name, session.uuid);
    println!("  mods    : {}", instance.mods_dir().display());
    if let Some(mc_instance::launch::QuickPlay::Multiplayer(hote)) = &launch_options.quick_play {
        println!("  serveur : {hote}");
    }
    println!();

    // Montrer sans lancer : c'est ce qu'on regarde quand le jeu refuse de
    // démarrer, et ce qui permet de rejouer la commande à la main.
    if afficher {
        println!("{}", command.display());
        return Ok(());
    }

    let status = mc_instance::launch::run(&command).await?;
    if !status.success() {
        bail!(
            "Minecraft s'est arrêté avec le code {} — journaux dans {}",
            status.code().unwrap_or(-1),
            instance.game_dir.join("logs").display()
        );
    }
    Ok(())
}
