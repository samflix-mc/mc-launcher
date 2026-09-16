//! Installe un pack, décrit par un manifeste JSON local ou publié.
//!
//!     mc-pack install                                  le pack publié, par défaut
//!     mc-pack install https://mc-launcher-dev.ggy.info/pack/samflix.json
//!     mc-pack install packs/samflix.json               un manifeste du dépôt
//!     mc-pack install packs/samflix.json --with-server installe aussi le serveur
//!     mc-pack lock    packs/samflix.json               résout sans installer le jeu
//!     mc-pack verify  [source] [--deep]
//!     mc-pack launch  [source] --pseudo Sam --serveur mc.exemple.fr
//!     mc-pack diagnostic                               journaux et télémétrie
//!
//! Sans argument, la source est le pack publié par mc-content et servi par
//! mc-launcher-site : c'est lui qui décide de la liste des mods, et un joueur
//! n'a donc rien à cloner. Un chemin reste accepté, c'est ce qu'on édite.
//!
//! **Lequel des trois packs** dépend de l'environnement de ce binaire, que la
//! CI lui fige à la compilation : un launcher de préproduction télécharge le
//! pack de préproduction. L'adresse était auparavant écrite en dur sur la
//! production, si bien qu'une préproduction n'éprouvait rien de ce qu'elle était
//! censée éprouver.
//!
//! Le pack désigne aussi le serveur à rejoindre, par environnement — **quand il
//! en désigne un**. La préproduction n'a pas de serveurs Minecraft derrière
//! elle, et le jeu s'y ouvre donc sur le menu.
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
use mc_pack::source::{self, Source};
use std::path::PathBuf;

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

    let mut source_arg: Option<String> = None;
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
            path => source_arg = Some(path.to_string()),
        }
    }

    // Le diagnostic n'a pas besoin de manifeste : c'est justement ce qu'on
    // lance quand on ne sait pas encore ce qui va de travers.
    if command == "diagnostic" {
        return diagnostic(&_log, incident_test);
    }

    // La source est construite après la boucle : `--data` peut déplacer la
    // racine des données, dont dépend l'emplacement du cache d'un pack distant.
    let source = Source::parse(
        source_arg.as_deref().unwrap_or(source::url_par_defaut()),
        &options.layout,
    );

    // Span racine : tout ce qui suit lui est rattaché, et sa durée est celle de
    // la commande. Dans le fichier comme dans Sentry, une exécution se lit ainsi
    // d'un bloc même quand plusieurs se succèdent.
    let span = tracing::info_span!(
        "commande",
        nom = %command,
        pack = %source.describe(),
        environnement = mc_log::environment::current().as_str(),
    );
    let _entree = span.enter();

    let debut = std::time::Instant::now();
    tracing::info!(
        environnement = mc_log::environment::current().as_str(),
        "mc-pack {command} sur {} — environnement {}",
        source.describe(),
        mc_log::environment::current().as_str()
    );

    let result = match command.as_str() {
        "install" => install(&source, &options).await,
        "launch" => launch(&source, &options, pseudo, serveur, memoire, afficher).await,
        "lock" => lock(&source, &options).await,
        "verify" => verify(&source, &options, deep),
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
         mc-pack install [source] [--locked] [--with-server] [--instance NOM] [--data DIR]\n  \
         mc-pack lock    <manifeste> [--data DIR]\n  \
         mc-pack verify  [source] [--deep] [--data DIR]\n  \
         mc-pack launch  [source] --pseudo NOM [--serveur HOTE[:PORT]] [--memoire MO] [--afficher]\n  \
         mc-pack diagnostic [--incident-test]\n\n\
         « source » est un chemin vers un manifeste, ou une URL.\n  \
         Par défaut : {}",
        source::url_par_defaut()
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

async fn install(source: &Source, options: &mc_pack::Options) -> Result<()> {
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

/// Résout et écrit le verrou sans installer le jeu.
///
/// C'est ce qu'on lance en revue : le verrou montre les versions retenues et
/// les dépendances ajoutées, sans attendre le téléchargement de huit cents
/// mégaoctets d'assets.
async fn lock(source: &Source, options: &mc_pack::Options) -> Result<()> {
    // Résoudre produit un verrou, et un verrou doit se poser quelque part.
    // Un pack publié n'offre pas cet endroit — et n'en a pas besoin : il
    // arrive déjà verrouillé, c'est justement ce qui en fait un pack publié.
    let Some(manifest_path) = source.local_path() else {
        bail!(
            "lock travaille sur un manifeste à éditer : donner un chemin.\n\
             Le pack publié est déjà verrouillé — c'est mc-content qui le résout."
        );
    };
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

fn verify(source: &Source, options: &mc_pack::Options, deep: bool) -> Result<()> {
    let problems = mc_pack::verify(source, options, deep)?;
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
    source: &Source,
    options: &mc_pack::Options,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
    afficher: bool,
) -> Result<()> {
    let Some(pseudo) = pseudo else {
        bail!("launch attend --pseudo <NOM>");
    };

    // Le pack posé sur cette machine, pas celui publié : on lance ce qui est
    // installé. Aller chercher le pack du jour décrirait des mods que le
    // dossier ne contient pas, et interdirait de jouer sans réseau.
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.ok_or_else(|| {
        anyhow::anyhow!(
            "{} absent : lancer « mc-pack install » avant de jouer",
            pack.lock_path.display()
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

    // À défaut de --serveur, celui que le pack déclare pour l'environnement de
    // ce binaire. Le manifeste est le même partout — c'est la même image de
    // contenu, servie sous trois noms — donc c'est au client de choisir, et il
    // choisit avec ce que la CI lui a figé à la compilation.
    //
    // Une absence n'est pas une erreur : la préproduction n'a pas de serveurs
    // Minecraft derrière elle, et le jeu s'y lance sans rejoindre quoi que ce
    // soit.
    let environnement = mc_log::environment::current();
    let demande_explicite = serveur.is_some();
    let cible = serveur.or_else(|| {
        manifest
            .server_for(environnement)
            .map(mc_pack::manifest::Server::address)
    });

    let launch_options = mc_instance::launch::LaunchOptions {
        memory_mb: memoire,
        quick_play: cible.map(mc_instance::launch::QuickPlay::Multiplayer),
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
    // La provenance est dite, pas seulement l'adresse : « mc.exemple.fr
    // (production) » laissait croire que l'hôte venait du pack, alors qu'un
    // --serveur peut désigner n'importe quoi. Quelqu'un qui diagnostique une
    // éjection a besoin de savoir lequel des deux il regarde.
    match (&launch_options.quick_play, demande_explicite) {
        (Some(mc_instance::launch::QuickPlay::Multiplayer(hote)), true) => {
            println!("  serveur : {hote} — demandé en ligne de commande")
        }
        (Some(mc_instance::launch::QuickPlay::Multiplayer(hote)), false) => println!(
            "  serveur : {hote} — déclaré par le pack pour « {} »",
            environnement.as_str()
        ),
        _ => println!(
            "  serveur : aucun pour « {} » — le jeu s'ouvrira sur le menu",
            environnement.as_str()
        ),
    }
    println!();

    // Montrer sans lancer : c'est ce qu'on regarde quand le jeu refuse de
    // démarrer, et ce qui permet de rejouer la commande à la main.
    if afficher {
        println!("{}", command.display());
        return Ok(());
    }

    // Daté avant le lancement : c'est ce qui permet d'écarter le rapport d'une
    // partie précédente, qui enverrait sur une fausse piste.
    let started_at = mc_instance::crash::now();

    let report = mc_instance::launch::run(&command).await?;

    // Remontées avant le verdict : une erreur survenue pendant une partie qui
    // s'est bien terminée compte autant qu'un plantage, et c'est elle qu'on ne
    // verrait jamais autrement.
    for erreur in &report.errors {
        report_game_error(&instance, &lock, &version_id, erreur, None);
    }
    if !report.errors.is_empty() {
        eprintln!(
            "\n{} erreurs relevées pendant la partie :",
            report.errors.len()
        );
        for erreur in &report.errors {
            eprintln!("  {} : {}", erreur.exception, erreur.message);
        }
    }

    match report.outcome {
        mc_instance::launch::Outcome::Normal => {
            tracing::info!("Minecraft s'est terminé normalement");
            Ok(())
        }
        // Fermer le jeu n'est pas une panne : le signaler comme telle
        // ouvrirait un incident à chaque partie terminée au clavier.
        mc_instance::launch::Outcome::Interrupted { signal } => {
            tracing::info!(signal, "Minecraft a été interrompu");
            println!("Minecraft a été interrompu (signal {signal}).");
            Ok(())
        }
        mc_instance::launch::Outcome::Failed { code } => {
            report_game_crash(&instance, &lock, &version_id, started_at, code);
            bail!(
                "Minecraft s'est arrêté avec le code {code} — journaux dans {}",
                instance.game_dir.join("logs").display()
            )
        }
    }
}

/// Cherche la cause d'un plantage du jeu et la remonte.
///
/// Sans cela, un incident ne porterait que « code de sortie 1 » : le launcher
/// et le jeu sont deux processus, et la trace Java reste du côté du jeu. C'est
/// pourtant le seul moment où elle est disponible — le joueur, lui, ne
/// l'enverra pas.
fn report_game_crash(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    started_at: std::time::SystemTime,
    code: i32,
) {
    let Some(crash) = mc_instance::crash::find(&instance.game_dir, started_at) else {
        // Aucune trace exploitable : l'incident du launcher reste, et il dit
        // au moins où chercher.
        tracing::warn!(
            code,
            journaux = %instance.game_dir.join("logs").display(),
            "Plantage sans trace exploitable dans les journaux du jeu"
        );
        return;
    };

    let id = report_game_error(instance, lock, version_id, &crash, Some(code));
    eprintln!("\n{} : {}", crash.exception, crash.message);
    eprintln!("  relevé dans {}", crash.source.display());
    if mc_log::telemetry_active() {
        eprintln!("  incident transmis sous l'identifiant {id}");
    }
}

/// Transmet une exception du jeu, qu'elle l'ait arrêté ou non.
///
/// Le contexte joint est celui qu'on demanderait sinon au joueur : versions,
/// mods présents, et d'où vient la trace.
fn report_game_error(
    instance: &mc_instance::Instance,
    lock: &Lockfile,
    version_id: &str,
    crash: &mc_instance::crash::Crash,
    code: Option<i32>,
) -> sentry::types::Uuid {
    let mods = mc_instance::crash::loaded_mods(&instance.game_dir).unwrap_or_default();
    let mut contexte = std::collections::BTreeMap::from([
        ("version".to_string(), version_id.to_string()),
        ("minecraft".to_string(), lock.minecraft.clone()),
        ("neoforge".to_string(), lock.loader.version.clone()),
        ("source".to_string(), crash.source.display().to_string()),
        ("mods".to_string(), mods.join("\n")),
    ]);
    if let Some(code) = code {
        contexte.insert("code_sortie".to_string(), code.to_string());
    } else {
        // Sans quoi rien ne distinguerait, dans le tableau de bord, une erreur
        // traversée d'une erreur fatale.
        contexte.insert("fatale".to_string(), "non".to_string());
    }

    tracing::error!(
        exception = %crash.exception,
        source = %crash.source.display(),
        fatale = code.is_some(),
        "Minecraft : {} : {}",
        crash.exception,
        crash.message
    );

    mc_log::capture_game_crash(&crash.exception, &crash.message, &crash.excerpt, &contexte)
}
