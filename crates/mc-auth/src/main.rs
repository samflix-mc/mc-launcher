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

/// Ce que la ligne de commande demande.
#[derive(Debug, PartialEq, Eq)]
enum Commande {
    Login,
    Whoami,
    Logout,
    HorsLigne(String),
}

/// Lecture des arguments, séparée de ce qu'ils déclenchent.
///
/// Trois des quatre commandes contactent Microsoft ou touchent au fichier de
/// session ; l'aiguillage, lui, se vérifie seul.
fn analyser(args: &[String]) -> Result<Commande> {
    match args.first().map(String::as_str) {
        Some("login") => Ok(Commande::Login),
        Some("whoami") => Ok(Commande::Whoami),
        Some("logout") => Ok(Commande::Logout),
        Some("--offline") => match args.get(1) {
            Some(pseudo) => Ok(Commande::HorsLigne(pseudo.clone())),
            None => bail!("usage : mc-auth --offline <PSEUDO>"),
        },
        _ => bail!("commande attendue"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Ce binaire manipule des jetons : la censure de mc-log s'applique à tout
    // ce qui sort, journal de fichier compris.
    let _log = mc_log::init("mc-auth");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let commande = match analyser(&args) {
        Ok(commande) => commande,
        Err(erreur) => {
            usage();
            return Err(erreur);
        }
    };

    match commande {
        Commande::Login => commandes::login().await,
        Commande::Whoami => commandes::whoami().await,
        Commande::Logout => commandes::logout(),
        Commande::HorsLigne(pseudo) => {
            commandes::hors_ligne(&pseudo);
            Ok(())
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

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;
