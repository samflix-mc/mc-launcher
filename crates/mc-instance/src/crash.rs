//! Ce que Minecraft laisse derrière lui quand il s'arrête mal.
//!
//! Un joueur dont le jeu plante ne sait pas lire une trace Java et ne pensera
//! pas à joindre un fichier. Le launcher, lui, sait exactement où regarder :
//!
//! - `crash-reports/crash-*.txt` — écrit par le jeu quand il attrape
//!   l'exception. Le plus riche : description, trace, mods chargés, pilote
//!   graphique ;
//! - `logs/latest.log` — le reste du temps. Une erreur de chargement de mods
//!   ou un conflit de modules s'y trouve, alors qu'aucun rapport n'est produit
//!   car la JVM s'arrête avant que le jeu n'existe.
//!
//! Le second cas est le plus fréquent avec un modpack, et c'est justement
//! celui qu'aucun rapport de crash ne couvre.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Ce qu'on a pu apprendre d'un arrêt anormal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crash {
    /// Type de l'exception Java, p. ex. `java.lang.module.ResolutionException`.
    ///
    /// C'est lui qui sert de titre : sans ça, tous les plantages du jeu se
    /// regrouperaient en un seul incident indistinct.
    pub exception: String,
    /// Message porté par l'exception.
    pub message: String,
    /// Extrait du fichier, borné pour rester lisible et envoyable.
    pub excerpt: String,
    /// Fichier d'où vient l'information.
    pub source: PathBuf,
}

/// Nombre de lignes conservées autour de l'exception.
///
/// Assez pour la trace et le contexte immédiat, pas assez pour dépasser les
/// limites d'un événement Sentry ni pour être illisible.
const EXCERPT_LINES: usize = 60;

/// Cherche de quoi expliquer un arrêt anormal.
///
/// `started_at` écarte les rapports d'une partie précédente : un crash vieux
/// de trois jours attribué au lancement du jour enverrait sur une fausse piste.
pub fn find(game_dir: &Path, started_at: std::time::SystemTime) -> Option<Crash> {
    latest_crash_report(game_dir, started_at)
        .and_then(|path| parse_file(&path))
        .or_else(|| {
            let log = game_dir.join("logs").join("latest.log");
            recent_enough(&log, started_at).then(|| parse_file(&log))?
        })
}

/// Rapport de crash le plus récent, s'il date de cette exécution.
fn latest_crash_report(game_dir: &Path, started_at: std::time::SystemTime) -> Option<PathBuf> {
    let dir = game_dir.join("crash-reports");
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("crash-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(modified, _)| *modified >= started_at)
        .collect();

    candidates.sort_by_key(|(modified, _)| *modified);
    candidates.pop().map(|(_, path)| path)
}

fn recent_enough(path: &Path, started_at: std::time::SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|modified| modified >= started_at)
        .unwrap_or(false)
}

fn parse_file(path: &Path) -> Option<Crash> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut crash = parse(&text)?;
    crash.source = path.to_path_buf();
    Some(crash)
}

/// Extrait la première exception d'un texte de journal.
///
/// La *première* et non la dernière : les suivantes en découlent souvent
/// (`Caused by`, exceptions de fermeture), et c'est celle d'origine qui
/// identifie le problème.
pub fn parse(text: &str) -> Option<Crash> {
    let lines: Vec<&str> = text.lines().collect();

    let (index, exception, message) = lines.iter().enumerate().find_map(|(i, line)| {
        let (exception, message) = split_exception(line)?;
        Some((i, exception, message))
    })?;

    // L'extrait démarre un peu avant : les lignes qui précèdent disent souvent
    // ce que le jeu était en train de faire.
    let start = index.saturating_sub(5);
    let end = (index + EXCERPT_LINES).min(lines.len());
    let excerpt = lines[start..end].join("\n");

    Some(Crash {
        exception,
        message,
        excerpt: strip_ansi(&excerpt),
        source: PathBuf::new(),
    })
}

/// Reconnaît `paquet.Classe: message` dans une ligne.
///
/// Une trace Java nomme sa classe par un chemin pointé finissant par un
/// identifiant capitalisé. Exiger les deux évite de prendre pour une exception
/// un horodatage ou un chemin de fichier, qui contiennent aussi des points et
/// des deux-points.
fn split_exception(line: &str) -> Option<(String, String)> {
    let line = strip_ansi(line);
    let line = line.trim();
    // Les lignes de trace commencent par « at » : ce sont des cadres, pas la
    // déclaration de l'exception.
    if line.starts_with("at ") {
        return None;
    }

    let candidate = match line.find("Caused by: ") {
        Some(pos) => &line[pos + "Caused by: ".len()..],
        None => line,
    };
    // Une ligne de journal préfixée d'un horodatage et d'une catégorie : on ne
    // garde que ce qui suit le dernier « ]: ».
    let candidate = match candidate.rfind("]: ") {
        Some(pos) => &candidate[pos + 3..],
        None => candidate,
    };

    let (class, message) = candidate.split_once(':')?;
    let class = class.trim();
    if !class.contains('.') || class.contains(' ') || class.contains('/') {
        return None;
    }
    let last = class.rsplit('.').next()?;
    if !last.chars().next()?.is_ascii_uppercase() {
        return None;
    }
    // Filet supplémentaire : une classe d'exception se termine presque
    // toujours ainsi, et s'en tenir là écarte les faux positifs restants.
    if !(last.ends_with("Exception") || last.ends_with("Error") || last.ends_with("Throwable")) {
        return None;
    }

    Some((class.to_string(), message.trim().to_string()))
}

/// Retire les séquences de couleur ANSI.
///
/// Minecraft colore sa sortie ; conservées, ces séquences rendent l'extrait
/// illisible dans un rapport d'incident.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Le jeu tourne-t-il encore ? Utilisé pour dater le début d'une exécution.
pub fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}

/// Repère les exceptions au fil de la sortie du jeu.
///
/// Attendre l'arrêt ne suffit pas : Minecraft rattrape beaucoup d'erreurs et
/// continue de tourner. Un mod qui échoue à charger sa configuration, une
/// texture absente, un gestionnaire d'événement qui lève — le jeu reste
/// jouable, l'erreur passe dans le journal, et personne ne la voit jamais.
/// Ce sont pourtant celles qui expliquent les comportements étranges qu'un
/// joueur signalera trois semaines plus tard.
#[derive(Debug, Default)]
pub struct Watcher {
    /// Exception en cours d'accumulation et nombre de lignes déjà prises.
    pending: Option<(Crash, usize)>,
    found: Vec<Crash>,
    /// Évite de remonter cent fois la même : un mod qui échoue à chaque tick
    /// remplirait le tableau de bord à lui seul.
    seen: std::collections::BTreeSet<String>,
}

/// Au-delà, on cesse de remonter : une session qui produit tant d'exceptions
/// distinctes a un problème global, que les premières décrivent déjà.
const MAX_DISTINCT: usize = 5;

impl Watcher {
    pub fn new() -> Watcher {
        Watcher::default()
    }

    /// Donne une ligne de la sortie du jeu.
    pub fn line(&mut self, line: &str) {
        let clean = strip_ansi(line);

        // Une trace se poursuit par ses cadres ; tant qu'ils arrivent, ils
        // appartiennent à l'exception en cours.
        if let Some((crash, taken)) = &mut self.pending {
            let trimmed = clean.trim_start();
            let continues = trimmed.starts_with("at ")
                || trimmed.starts_with("Caused by:")
                || trimmed.starts_with("... ")
                || trimmed.starts_with("Suppressed:");
            if continues && *taken < EXCERPT_LINES {
                crash.excerpt.push('\n');
                crash.excerpt.push_str(&clean);
                *taken += 1;
                return;
            }
            let (finished, _) = self.pending.take().expect("présent");
            self.keep(finished);
        }

        if self.found.len() >= MAX_DISTINCT {
            return;
        }
        if let Some((exception, message)) = split_exception(&clean) {
            self.pending = Some((
                Crash {
                    exception,
                    message,
                    excerpt: clean,
                    source: PathBuf::from("sortie du jeu"),
                },
                0,
            ));
        }
    }

    fn keep(&mut self, crash: Crash) {
        let key = format!("{}: {}", crash.exception, crash.message);
        if self.seen.insert(key) {
            self.found.push(crash);
        }
    }

    /// Exceptions distinctes relevées pendant l'exécution.
    pub fn finish(mut self) -> Vec<Crash> {
        if let Some((crash, _)) = self.pending.take() {
            self.keep(crash);
        }
        self.found
    }
}

/// Liste des mods chargés, pour accompagner un rapport.
///
/// Un plantage de modpack vient presque toujours d'un mod ou d'un couple de
/// mods ; savoir lesquels étaient présents épargne un aller-retour.
pub fn loaded_mods(game_dir: &Path) -> Result<Vec<String>> {
    let mods = game_dir.join("mods");
    let Ok(entries) = std::fs::read_dir(&mods) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jar"))
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_une_exception_de_resolution_de_modules() {
        // Le cas réel qui a empêché le premier lancement d'aboutir.
        let texte = "\
[04:01:48] [main/ERROR] [cpw.mods.modlauncher.Launcher/MODLAUNCHER]: Exception
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package com.mojang.blaze3d.systems to module bookshelf
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source) ~[?:?]
\tat java.base/java.lang.module.Resolver.failTwoSuppliers(Unknown Source) ~[?:?]";

        let crash = parse(texte).expect("exception trouvée");
        assert_eq!(crash.exception, "java.lang.module.ResolutionException");
        assert!(crash.message.starts_with("Modules _1._21._1 and minecraft"));
        assert!(crash.excerpt.contains("resolveFail"));
    }

    #[test]
    fn la_premiere_exception_l_emporte() {
        // Les suivantes découlent souvent de la première.
        let texte = "\
java.lang.NullPointerException: rien
\tat quelque.part(Chose.java:1)
Caused by: java.lang.IllegalStateException: conséquence";
        assert_eq!(
            parse(texte).unwrap().exception,
            "java.lang.NullPointerException"
        );
    }

    #[test]
    fn un_caused_by_seul_est_reconnu() {
        let texte = "Caused by: java.io.IOException: disque plein";
        let crash = parse(texte).unwrap();
        assert_eq!(crash.exception, "java.io.IOException");
        assert_eq!(crash.message, "disque plein");
    }

    #[test]
    fn un_journal_ordinaire_ne_produit_pas_de_crash() {
        // Sans ce filtre, chaque lancement réussi remonterait un faux incident.
        let texte = "\
[04:01:47] [main/INFO] [Launcher/MODLAUNCHER]: ModLauncher running: args [--username, Sam]
[04:01:48] [main/INFO] [ModDiscoverer/SCAN]: Found mod file \"jei-1.21.1.jar\"
[04:01:50] [Render thread/INFO] [minecraft/Minecraft]: Setting user: Sam";
        assert!(parse(texte).is_none());
    }

    #[test]
    fn un_horodatage_n_est_pas_pris_pour_une_exception() {
        // « 04:01:47 » contient des deux-points et des chiffres.
        assert!(split_exception("[04:01:47] [main/INFO] démarrage").is_none());
    }

    #[test]
    fn un_chemin_de_fichier_n_est_pas_pris_pour_une_exception() {
        assert!(split_exception("fichier: /home/sam/.local/share/truc.jar").is_none());
    }

    #[test]
    fn une_ligne_de_trace_n_est_pas_une_declaration() {
        assert!(split_exception("\tat java.base/java.lang.Thread.run(Thread.java:1)").is_none());
    }

    #[test]
    fn les_couleurs_sont_retirees() {
        let colore = "\u{1b}[32m[04:01] ERREUR\u{1b}[m suite";
        assert_eq!(strip_ansi(colore), "[04:01] ERREUR suite");
    }

    #[test]
    fn une_exception_coloree_reste_reconnaissable() {
        // Minecraft colore sa sortie ; sans le nettoyage, rien ne correspond.
        let ligne = "\u{1b}[31mjava.lang.OutOfMemoryError: Java heap space\u{1b}[m";
        let (exception, message) = split_exception(ligne).unwrap();
        assert_eq!(exception, "java.lang.OutOfMemoryError");
        assert_eq!(message, "Java heap space");
    }

    #[test]
    fn une_classe_qui_n_est_pas_une_exception_est_ecartee() {
        // « net.minecraft.client.Minecraft: démarrage » n'est pas un plantage.
        assert!(split_exception("net.minecraft.client.Minecraft: démarrage").is_none());
    }
}
