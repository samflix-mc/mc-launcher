//! Vérifie qu'un Java utilisable est disponible, et l'installe sinon.
//!
//!     mc-java              détecte un Java 21, l'installe s'il n'y en a pas
//!     mc-java --check      détecte seulement, code de sortie 1 si absent
//!     mc-java --major 17   autre version majeure
//!     mc-java --dir <DIR>  autre répertoire de runtimes

use anyhow::{Result, bail};
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

async fn run() -> Result<ExitCode> {
    let mut major = 21;
    let mut check_only = false;
    let mut dir: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--major" => {
                major = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| panic!("--major attend un entier"));
            }
            "--dir" => dir = args.next().map(PathBuf::from),
            other => bail!("option inconnue : {other}"),
        }
    }

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
