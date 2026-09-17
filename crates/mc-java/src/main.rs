//! Vérifie qu'un Java utilisable est disponible, et l'installe sinon.
//!
//!     mc-java              détecte un Java 21, l'installe s'il n'y en a pas
//!     mc-java --check      détecte seulement, code de sortie 1 si absent
//!     mc-java --major 17   autre version majeure
//!     mc-java --dir <DIR>  autre répertoire de runtimes

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Le guard vit jusqu'au retour de `main` pour que le journal se vide : un
    // `std::process::exit` au milieu de `run` le laisserait dans la file.
    let _log = mc_log::init("mc-java");

    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Erreur : {error:?}");
            ExitCode::FAILURE
        }
    }
}

/// Ce que la ligne de commande demande.
#[derive(Debug, PartialEq, Eq)]
struct Reglages {
    major: u32,
    check_only: bool,
    dir: Option<PathBuf>,
}

impl Default for Reglages {
    fn default() -> Self {
        // Minecraft 1.21.1 exige Java 21 : en dessous, le jeu s'arrête sur
        // `UnsupportedClassVersionError` avant même d'afficher une fenêtre.
        Reglages {
            major: 21,
            check_only: false,
            dir: None,
        }
    }
}

/// Lecture des arguments, séparée de ce qu'ils déclenchent.
fn analyser(args: impl Iterator<Item = String>) -> Result<Reglages> {
    let mut reglages = Reglages::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => reglages.check_only = true,
            "--major" => {
                // Une valeur illisible s'annonce comme une erreur ordinaire :
                // une panique afficherait une trace d'appels là où il n'y a
                // qu'une faute de frappe.
                let brut = args.next().context("--major attend un entier")?;
                reglages.major = brut
                    .parse()
                    .with_context(|| format!("--major attend un entier, reçu « {brut} »"))?;
            }
            "--dir" => {
                reglages.dir = Some(PathBuf::from(
                    args.next().context("--dir attend un chemin")?,
                ));
            }
            other => bail!("option inconnue : {other}"),
        }
    }
    Ok(reglages)
}

async fn run() -> Result<ExitCode> {
    let Reglages {
        major,
        check_only,
        dir,
    } = analyser(std::env::args().skip(1))?;

    executer(major, check_only, dir).await
}

/// Ce que les réglages déclenchent, séparé de leur lecture.
///
/// `--check` est le seul chemin qui ne touche à rien : il dit si ce poste a
/// déjà un Java utilisable, et c'est celui qu'une CI appelle.
async fn executer(major: u32, check_only: bool, dir: Option<PathBuf>) -> Result<ExitCode> {
    let runtime_dir = dir.unwrap_or_else(mc_java::default_runtime_dir);

    if let Some(java) = mc_java::detect(major, &runtime_dir).await {
        println!(
            "Java {} trouvé ({}) — {}",
            java.version.full,
            match java.origin {
                mc_java::Origin::Managed => "installé par le launcher",
                mc_java::Origin::System => "runtime du système",
            },
            java.path.display()
        );
        return Ok(ExitCode::SUCCESS);
    }

    if check_only {
        eprintln!("Aucun Java {major} ou supérieur sur ce poste.");
        return Ok(ExitCode::FAILURE);
    }

    println!("Aucun Java {major} détecté, installation de Temurin {major}…");
    let java = mc_java::install(major, &runtime_dir).await?;
    println!(
        "Java {} installé — {}",
        java.version.full,
        java.path.display()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;
