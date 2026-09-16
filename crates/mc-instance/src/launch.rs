//! Construire la ligne de commande qui démarre le jeu.
//!
//! Minecraft ne se lance pas : il se *compose*. Le descripteur de NeoForge ne
//! contient qu'un delta et désigne son socle par `inheritsFrom` ; il faut
//! fusionner les deux, choisir les bibliothèques valables pour ce système,
//! assembler un classpath, puis remplacer une vingtaine de variables dans des
//! arguments dont certains n'apparaissent que sous condition.
//!
//! Quatre points décident que le jeu démarre ou non :
//!
//! - **l'ordre du classpath** — NeoForge remplace certaines bibliothèques de
//!   Mojang. Sa version doit passer devant, sinon la JVM charge celle de
//!   Mojang et le chargeur échoue sur une méthode absente ;
//! - **le client vanilla** — NeoForge ne le déclare pas parmi ses
//!   bibliothèques. Il est ajouté au classpath, et c'est FML qui le transforme
//!   au chargement ;
//! - **les natives** — inutile de les extraire : les arguments de Mojang
//!   passent `org.lwjgl.system.SharedLibraryExtractPath`, et LWJGL 3.3 sort
//!   lui-même ses binaires des jars du classpath. Il suffit que le répertoire
//!   existe ;
//! - **les drapeaux** — `--quickPlayMultiplayer` n'existe dans le descripteur
//!   que derrière une règle `is_quick_play_multiplayer`. Ignorer les règles de
//!   drapeaux produit une ligne de commande que le jeu refuse.

use crate::vanilla::{self, Features, Library};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Identité du joueur transmise au jeu.
///
/// Volontairement indépendante de `mc-auth` : une session hors-ligne et une
/// session Microsoft produisent la même structure, et ce module n'a pas à
/// savoir laquelle il sert.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub uuid: String,
    /// Jeton d'accès. Vide en hors-ligne — le jeu l'accepte et ne rejoint
    /// alors que des serveurs en `online-mode=false`.
    pub token: String,
    /// `msa` pour un compte Microsoft, `legacy` sinon.
    pub user_type: String,
    pub xuid: String,
    pub client_id: String,
}

impl Session {
    /// Session hors-ligne, pour un serveur en `online-mode=false`.
    pub fn offline(name: impl Into<String>, uuid: impl Into<String>) -> Session {
        Session {
            name: name.into(),
            uuid: uuid.into(),
            // Le jeu exige l'argument mais ne le valide pas hors ligne. La
            // valeur « 0 » est celle qu'emploient les launchers usuels : une
            // chaîne vide ferait échouer l'analyse des arguments.
            token: "0".into(),
            user_type: "legacy".into(),
            xuid: String::new(),
            client_id: String::new(),
        }
    }
}

/// Partie à rejoindre directement au démarrage.
#[derive(Debug, Clone)]
pub enum QuickPlay {
    /// Serveur, au format `hôte` ou `hôte:port`.
    Multiplayer(String),
    /// Monde local, par son nom de dossier.
    Singleplayer(String),
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Mémoire maximale de la JVM, en mébioctets.
    pub memory_mb: Option<u32>,
    pub quick_play: Option<QuickPlay>,
    pub resolution: Option<(u32, u32)>,
    /// Arguments JVM ajoutés avant ceux du descripteur.
    pub extra_jvm: Vec<String>,
}

/// Ce qu'il faut exécuter pour démarrer le jeu.
#[derive(Debug, Clone)]
pub struct Command {
    pub java: PathBuf,
    pub args: Vec<String>,
    /// Répertoire de travail : le jeu y écrit `saves`, `logs`, `options.txt`.
    pub working_dir: PathBuf,
}

impl Command {
    /// Ligne de commande lisible, pour l'afficher ou la rejouer à la main.
    ///
    /// Le classpath est abrégé : il fait plusieurs dizaines de milliers de
    /// caractères et personne ne le lit.
    pub fn display(&self) -> String {
        let mut out = vec![self.java.display().to_string()];
        let mut skip_next = false;
        for arg in &self.args {
            if skip_next {
                out.push(format!("<{} bibliothèques>", arg.split(':').count()));
                skip_next = false;
                continue;
            }
            if arg == "-cp" {
                skip_next = true;
            }
            out.push(arg.clone());
        }
        out.join(" ")
    }
}

// --- Lecture des descripteurs ------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionJson {
    id: String,
    inherits_from: Option<String>,
    main_class: Option<String>,
    asset_index: Option<AssetIndex>,
    assets: Option<String>,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    arguments: Arguments,
    /// Format d'avant 2017, encore présent sur les très vieilles versions.
    minecraft_arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    id: String,
}

#[derive(Debug, Default, Deserialize)]
struct Arguments {
    #[serde(default)]
    game: Vec<Argument>,
    #[serde(default)]
    jvm: Vec<Argument>,
}

/// Un argument : soit une chaîne, soit une valeur sous condition.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Argument {
    Simple(String),
    Conditional {
        #[serde(default)]
        rules: Vec<vanilla::Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

impl ArgumentValue {
    fn parts(&self) -> &[String] {
        match self {
            ArgumentValue::One(v) => std::slice::from_ref(v),
            ArgumentValue::Many(v) => v,
        }
    }
}

fn read_version(shared: &Path, id: &str) -> Result<VersionJson> {
    let path = shared.join("versions").join(id).join(format!("{id}.json"));
    let raw = std::fs::read(&path)
        .with_context(|| format!("{id} n'est pas installé : {} absent", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("{} illisible", path.display()))
}

/// Descripteur complet d'une version, son socle fusionné.
///
/// La chaîne d'héritage est suivie sur plusieurs niveaux par prudence, même si
/// NeoForge n'en compte qu'un — rien ne garantit qu'un autre chargeur s'en
/// tienne là.
fn resolve_chain(shared: &Path, id: &str) -> Result<Vec<VersionJson>> {
    let mut chain = Vec::new();
    let mut current = id.to_string();
    for _ in 0..8 {
        let version = read_version(shared, &current)?;
        let parent = version.inherits_from.clone();
        chain.push(version);
        match parent {
            Some(next) => current = next,
            None => return Ok(chain),
        }
    }
    bail!("chaîne d'héritage de versions trop profonde à partir de {id}")
}

// --- Construction ------------------------------------------------------------

/// Clé d'unicité d'une bibliothèque : groupe, artefact et classifier.
///
/// La version en est exclue volontairement — c'est ce qui permet de repérer
/// que NeoForge remplace une bibliothèque de Mojang par une autre version.
fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0 | 1 => name.to_string(),
        2 | 3 => format!("{}:{}", parts[0], parts[1]),
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}

/// Construit la ligne de commande.
#[tracing::instrument(
    name = "ligne de commande",
    skip(shared, game_dir, java, session, options),
    fields(version = version_id)
)]
pub fn build(
    version_id: &str,
    shared: &Path,
    game_dir: &Path,
    java: &Path,
    session: &Session,
    options: &LaunchOptions,
) -> Result<Command> {
    let chain = resolve_chain(shared, version_id)?;
    let base = chain.last().expect("au moins une version");

    let main_class = chain
        .iter()
        .find_map(|v| v.main_class.clone())
        .context("aucune classe principale dans la chaîne de versions")?;
    let assets_index = chain
        .iter()
        .find_map(|v| v.asset_index.as_ref().map(|a| a.id.clone()))
        .or_else(|| chain.iter().find_map(|v| v.assets.clone()))
        .context("aucun index d'assets dans la chaîne de versions")?;

    let os = vanilla::mojang_os();
    let arch = vanilla::mojang_arch();
    let features = active_features(options);

    // --- Classpath ---
    // Parcours du plus spécifique au plus général : NeoForge d'abord, Mojang
    // ensuite. La première occurrence d'une bibliothèque gagne, donc une
    // version remplacée par le chargeur prend la place de celle du jeu.
    let libraries_root = shared.join("libraries");
    let mut seen = BTreeMap::new();
    let mut classpath: Vec<PathBuf> = Vec::new();
    for version in &chain {
        for library in &version.libraries {
            if !vanilla::allowed(&library.rules, os, arch) {
                continue;
            }
            let key = library_key(&library.name);
            if seen.contains_key(&key) {
                continue;
            }
            let Some(relative) = library
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.clone())
                .or_else(|| vanilla::maven_path(&library.name))
            else {
                bail!("bibliothèque sans chemin exploitable : {}", library.name);
            };
            seen.insert(key, library.name.clone());
            classpath.push(libraries_root.join(relative));
        }
    }

    // Le client de Mojang ne rejoint le classpath que pour du vanilla pur.
    //
    // Sous un chargeur, l'installateur a produit sa propre découpe du client —
    // `client-…-slim.jar` pour le code, `client-…-extra.jar` pour les
    // ressources — et FML les résout lui-même à partir de `libraryDirectory`.
    // Ajouter `1.21.1.jar` par-dessus donne deux modules qui exportent les
    // mêmes paquets, et la JVM s'arrête avant le premier écran :
    //
    //     java.lang.module.ResolutionException: Modules _1._21._1 and
    //     minecraft export package com.mojang.blaze3d.systems to module …
    //
    // Le nom `_1._21._1` est celui que la JVM dérive de `1.21.1.jar` : il
    // désigne sans ambiguïté le jar ajouté ici, et c'est ce qui a permis de
    // retrouver la cause.
    let uses_loader = chain.len() > 1;
    let client_jar = shared
        .join("versions")
        .join(&base.id)
        .join(format!("{}.jar", base.id));
    if !client_jar.is_file() {
        bail!("client absent : {}", client_jar.display());
    }
    if uses_loader {
        tracing::debug!(
            client = %client_jar.display(),
            "client vanilla laissé hors du classpath, le chargeur fournit le sien"
        );
    } else {
        classpath.push(client_jar);
    }

    let missing: Vec<&PathBuf> = classpath.iter().filter(|p| !p.is_file()).collect();
    if let Some(first) = missing.first() {
        bail!(
            "{} bibliothèques manquantes, à commencer par {} — relancer l'installation",
            missing.len(),
            first.display()
        );
    }

    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath_text = classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    // LWJGL y dépose les binaires qu'il sort des jars ; il refuse de démarrer
    // si le répertoire n'existe pas.
    let natives = shared.join("natives").join(&base.id);
    std::fs::create_dir_all(&natives)
        .with_context(|| format!("création de {}", natives.display()))?;
    std::fs::create_dir_all(game_dir)
        .with_context(|| format!("création de {}", game_dir.display()))?;

    let variables = variables(
        version_id,
        &base.id,
        game_dir,
        shared,
        &assets_index,
        &natives,
        &libraries_root,
        &classpath_text,
        separator,
        session,
        options,
    );

    // --- Arguments ---
    // Le socle pose les siens, le chargeur ajoute les siens par-dessus : la
    // chaîne est donc parcourue à l'envers de l'héritage.
    let mut jvm = Vec::new();
    let mut game = Vec::new();
    for version in chain.iter().rev() {
        collect(
            &version.arguments.jvm,
            os,
            arch,
            &features,
            &variables,
            &mut jvm,
        );
        collect(
            &version.arguments.game,
            os,
            arch,
            &features,
            &variables,
            &mut game,
        );
        if let Some(legacy) = &version.minecraft_arguments {
            game.extend(legacy.split_whitespace().map(|a| substitute(a, &variables)));
        }
    }

    if jvm.is_empty() {
        // Format d'avant 2017 : le descripteur ne décrit pas les arguments JVM.
        jvm.push("-Djava.library.path=".to_string() + &natives.display().to_string());
        jvm.push("-cp".into());
        jvm.push(classpath_text.clone());
    }

    let mut args = Vec::new();
    if let Some(mb) = options.memory_mb {
        // Avant ceux du descripteur, pour qu'un réglage explicite puisse être
        // contredit par ce que le chargeur impose s'il y tient.
        args.push(format!("-Xmx{mb}M"));
    }
    args.extend(options.extra_jvm.iter().cloned());
    args.extend(jvm);
    args.push(main_class);
    args.extend(game);

    tracing::debug!(
        bibliotheques = classpath.len(),
        arguments = args.len(),
        "ligne de commande assemblée"
    );

    Ok(Command {
        java: java.to_path_buf(),
        args,
        working_dir: game_dir.to_path_buf(),
    })
}

/// Drapeaux actifs, déduits des options demandées.
fn active_features(options: &LaunchOptions) -> Features {
    let mut features = Features::new();
    if options.resolution.is_some() {
        features.insert("has_custom_resolution".into());
    }
    if options.quick_play.is_some() {
        // `has_quick_plays_support` conditionne `--quickPlayPath`, le journal
        // que le jeu écrit ; les deux autres désignent la destination.
        features.insert("has_quick_plays_support".into());
        match options.quick_play {
            Some(QuickPlay::Multiplayer(_)) => {
                features.insert("is_quick_play_multiplayer".into());
            }
            Some(QuickPlay::Singleplayer(_)) => {
                features.insert("is_quick_play_singleplayer".into());
            }
            None => {}
        }
    }
    features
}

#[allow(clippy::too_many_arguments)]
fn variables(
    version_id: &str,
    base_id: &str,
    game_dir: &Path,
    shared: &Path,
    assets_index: &str,
    natives: &Path,
    libraries: &Path,
    classpath: &str,
    separator: &str,
    session: &Session,
    options: &LaunchOptions,
) -> BTreeMap<String, String> {
    let mut v = BTreeMap::new();
    let mut set = |k: &str, value: String| {
        v.insert(k.to_string(), value);
    };

    set("auth_player_name", session.name.clone());
    set("auth_uuid", session.uuid.clone());
    set("auth_access_token", session.token.clone());
    set("auth_xuid", session.xuid.clone());
    set("clientid", session.client_id.clone());
    set("user_type", session.user_type.clone());
    set("auth_session", format!("token:{}", session.token));

    set("version_name", version_id.to_string());
    set("version_type", "release".into());
    set("game_directory", game_dir.display().to_string());
    set("assets_root", shared.join("assets").display().to_string());
    set("game_assets", shared.join("assets").display().to_string());
    set("assets_index_name", assets_index.to_string());
    set("natives_directory", natives.display().to_string());
    set("library_directory", libraries.display().to_string());
    set("classpath", classpath.to_string());
    set("classpath_separator", separator.to_string());
    set("launcher_name", "samflix-mc".into());
    set("launcher_version", env!("CARGO_PKG_VERSION").to_string());
    // Le jar du socle, que NeoForge nomme dans son `ignoreList`.
    set("primary_jar_name", format!("{base_id}.jar"));

    if let Some((width, height)) = options.resolution {
        set("resolution_width", width.to_string());
        set("resolution_height", height.to_string());
    }
    match &options.quick_play {
        Some(QuickPlay::Multiplayer(target)) => {
            set("quickPlayMultiplayer", target.clone());
            set(
                "quickPlayPath",
                game_dir.join("quickPlay.json").display().to_string(),
            );
        }
        Some(QuickPlay::Singleplayer(world)) => {
            set("quickPlaySingleplayer", world.clone());
            set(
                "quickPlayPath",
                game_dir.join("quickPlay.json").display().to_string(),
            );
        }
        None => {}
    }
    v
}

fn collect(
    arguments: &[Argument],
    os: &str,
    arch: &str,
    features: &Features,
    variables: &BTreeMap<String, String>,
    out: &mut Vec<String>,
) {
    for argument in arguments {
        match argument {
            Argument::Simple(value) => out.push(substitute(value, variables)),
            Argument::Conditional { rules, value } => {
                if !vanilla::allowed_with(rules, os, arch, features) {
                    continue;
                }
                for part in value.parts() {
                    out.push(substitute(part, variables));
                }
            }
        }
    }
}

/// Remplace les `${…}` par leur valeur.
///
/// Une variable inconnue est laissée telle quelle plutôt que vidée : un
/// argument qui garde `${quelque_chose}` se remarque dans un message d'erreur,
/// là où un argument devenu vide décale silencieusement tous les suivants.
fn substitute(text: &str, variables: &BTreeMap<String, String>) -> String {
    if !text.contains("${") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match variables.get(name) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push_str("${");
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Lance le jeu et attend qu'il se termine.
#[tracing::instrument(name = "exécution du jeu", skip_all)]
pub async fn run(command: &Command) -> Result<Report> {
    tracing::info!(
        java = %command.java.display(),
        repertoire = %command.working_dir.display(),
        arguments = command.args.len(),
        "Démarrage de Minecraft"
    );

    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // La sortie est captée pour être lue, puis réécrite telle quelle : le
    // joueur voit ce qu'il aurait vu, et le launcher peut en tirer les
    // exceptions au passage. Les deux flux sont fusionnés parce que Minecraft
    // écrit sur les deux sans distinction utile.
    let mut child = tokio::process::Command::new(&command.java)
        .args(&command.args)
        .current_dir(&command.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("exécution de {}", command.java.display()))?;

    let mut watcher = crate::crash::Watcher::new();
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    let mut out_lines = stdout.map(|r| r.lines());
    let mut err_lines = stderr.map(|r| r.lines());

    loop {
        let line = tokio::select! {
            Ok(Some(line)) = async {
                match &mut out_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            Ok(Some(line)) = async {
                match &mut err_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            else => None,
        };
        match line {
            Some(line) => {
                println!("{line}");
                watcher.line(&line);
            }
            None => break,
        }
    }

    let status = child
        .wait()
        .await
        .with_context(|| format!("attente de {}", command.java.display()))?;

    let outcome = Outcome::from_status(&status);
    // Rien n'est journalisé en erreur ici : c'est l'appelant qui décide, et
    // c'est lui qui connaît le contexte. Le faire aux deux endroits produisait
    // deux incidents distincts pour un seul échec, constaté sur MC-LAUNCHER-4
    // et MC-LAUNCHER-5.
    tracing::debug!(?outcome, code = status.code(), "Minecraft s'est terminé");

    Ok(Report {
        outcome,
        errors: watcher.finish(),
    })
}

/// Ce qu'une exécution du jeu a produit.
#[derive(Debug)]
pub struct Report {
    pub outcome: Outcome,
    /// Exceptions relevées dans la sortie, que le jeu ait planté ou non.
    ///
    /// Minecraft en rattrape beaucoup et continue : ces erreurs-là
    /// n'apparaissent nulle part ailleurs, et ce sont souvent elles qui
    /// expliquent un comportement signalé bien plus tard.
    pub errors: Vec<crate::crash::Crash>,
}

/// Comment le jeu s'est terminé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Sortie normale, écran de fin ou fermeture de la fenêtre.
    Normal,
    /// Arrêt demandé de l'extérieur : `Ctrl+C`, `kill`, fermeture de session.
    ///
    /// N'est pas une panne. Le confondre avec une erreur remplissait le
    /// tableau de bord d'incidents à chaque fois qu'on fermait le jeu.
    Interrupted { signal: i32 },
    /// Le jeu s'est arrêté de lui-même sur une erreur.
    Failed { code: i32 },
}

impl Outcome {
    fn from_status(status: &std::process::ExitStatus) -> Outcome {
        if status.success() {
            return Outcome::Normal;
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Outcome::Interrupted { signal };
            }
            // Un shell traduit un signal en 128 + n. `status.signal()` ne le
            // voit pas quand le code traverse un intermédiaire, d'où cette
            // seconde lecture : 143 est un SIGTERM, 130 un Ctrl+C.
            if let Some(code) = status.code()
                && (129..=192).contains(&code)
            {
                return Outcome::Interrupted { signal: code - 128 };
            }
        }

        Outcome::Failed {
            code: status.code().unwrap_or(-1),
        }
    }

    /// Y a-t-il matière à ouvrir un incident ?
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("auth_player_name".into(), "Sam".into()),
            ("game_directory".into(), "/jeu".into()),
            ("classpath_separator".into(), ":".into()),
        ])
    }

    #[test]
    fn substitution_simple() {
        assert_eq!(substitute("${auth_player_name}", &vars()), "Sam");
        assert_eq!(
            substitute("--gameDir=${game_directory}", &vars()),
            "--gameDir=/jeu"
        );
    }

    #[test]
    fn plusieurs_variables_dans_un_argument() {
        // Le module path de NeoForge en enchaîne une dizaine.
        let rendu = substitute("a${classpath_separator}b${classpath_separator}c", &vars());
        assert_eq!(rendu, "a:b:c");
    }

    #[test]
    fn une_variable_inconnue_reste_visible() {
        // La vider décalerait les arguments suivants sans rien signaler.
        assert_eq!(substitute("${inconnue}", &vars()), "${inconnue}");
    }

    #[test]
    fn un_texte_sans_variable_est_intact() {
        assert_eq!(substitute("--add-modules", &vars()), "--add-modules");
    }

    #[test]
    fn une_accolade_non_fermee_ne_fait_pas_paniquer() {
        assert_eq!(substitute("${tronque", &vars()), "${tronque");
    }

    #[test]
    fn la_cle_de_bibliotheque_ignore_la_version() {
        // C'est ce qui permet de voir qu'une bibliothèque en remplace une autre.
        assert_eq!(
            library_key("com.google.guava:guava:32.1.2-jre"),
            "com.google.guava:guava"
        );
        assert_eq!(
            library_key("com.google.guava:guava:31.0-jre"),
            library_key("com.google.guava:guava:32.1.2-jre")
        );
    }

    #[test]
    fn le_classifier_distingue_deux_bibliotheques() {
        // lwjgl et lwjgl:natives-linux sont deux fichiers, tous deux nécessaires.
        assert_ne!(
            library_key("org.lwjgl:lwjgl:3.3.3"),
            library_key("org.lwjgl:lwjgl:3.3.3:natives-linux")
        );
    }

    #[test]
    fn les_drapeaux_suivent_les_options() {
        let sans = active_features(&LaunchOptions::default());
        assert!(sans.is_empty());

        let avec = active_features(&LaunchOptions {
            quick_play: Some(QuickPlay::Multiplayer("mc.exemple.fr".into())),
            resolution: Some((1280, 720)),
            ..Default::default()
        });
        assert!(avec.contains("is_quick_play_multiplayer"));
        assert!(avec.contains("has_quick_plays_support"));
        assert!(avec.contains("has_custom_resolution"));
        assert!(!avec.contains("is_quick_play_singleplayer"));
    }

    #[test]
    fn une_session_hors_ligne_porte_un_jeton_non_vide() {
        // Le jeu exige l'argument ; une chaîne vide casse l'analyse.
        let session = Session::offline("Sam", "uuid");
        assert!(!session.token.is_empty());
        assert_eq!(session.user_type, "legacy");
    }
}
