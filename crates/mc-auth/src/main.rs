//! Gère la session Microsoft du launcher, et permet de la vérifier.
//!
//!     mc-auth login              ouvre une session et l'enregistre
//!     mc-auth whoami             affiche la session enregistrée
//!     mc-auth logout             oublie la session
//!     mc-auth --offline <PSEUDO> profil local, sans Microsoft
//!
//! La connexion présente l'identité du launcher officiel : voir la doc du
//! crate et le README pour ce que ce choix implique.

use anyhow::{Result, bail};

mod commandes;

#[tokio::main]
async fn main() -> Result<()> {
    // Ce binaire manipule des jetons : la censure de mc-log s'applique à tout
    // ce qui sort, journal de fichier compris.
    let _log = mc_log::init("mc-auth");

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("login") => commandes::login().await,
        Some("whoami") => commandes::whoami().await,
        Some("logout") => commandes::logout(),
        Some("--offline") => {
            let Some(pseudo) = args.get(1) else {
                bail!("usage : mc-auth --offline <PSEUDO>");
            };
            commandes::hors_ligne(pseudo);
            Ok(())
        }
        _ => {
            usage();
            bail!("commande attendue");
        }
    }
}

fn usage() {
    eprintln!("usage :");
    eprintln!("  mc-auth login              ouvre une session et l'enregistre");
    eprintln!("  mc-auth whoami             affiche la session enregistrée");
    eprintln!("  mc-auth logout             oublie la session");
    eprintln!("  mc-auth --offline <PSEUDO> profil local, sans Microsoft");
}
