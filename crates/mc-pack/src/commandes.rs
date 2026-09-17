//! Une commande par module, comme la page d'aide les énumère.

pub mod arguments;

#[cfg(test)]
mod essais;

pub mod diagnostic;
pub mod install;
pub mod lancement;
pub mod lock;
pub mod usage;
pub mod verify;

pub use diagnostic::diagnostic;
pub use install::install;
pub use lancement::launch;
pub use lock::lock;
pub use usage::usage;
pub use verify::verify;

use anyhow::Result;
use mc_pack::source::Source;
use std::process::ExitCode;

/// Aiguille vers la commande demandée.
///
/// `verify` est la seule à distinguer deux réussites : l'installation est
/// conforme, ou elle ne l'est pas sans que le programme ait pour autant échoué.
#[allow(clippy::too_many_arguments)]
pub async fn executer(
    command: &str,
    source: &Source,
    options: &mc_pack::Options,
    deep: bool,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
    afficher: bool,
) -> Result<ExitCode> {
    match command {
        "install" => install(source, options).await.map(|()| ExitCode::SUCCESS),
        "launch" => launch(source, options, pseudo, serveur, memoire, afficher)
            .await
            .map(|()| ExitCode::SUCCESS),
        "lock" => lock(source, options).await.map(|()| ExitCode::SUCCESS),
        "verify" => verify(source, options, deep),
        other => {
            eprintln!("commande inconnue : {other}");
            usage();
            Ok(ExitCode::from(2))
        }
    }
}

#[cfg(test)]
#[path = "commandes.test.rs"]
mod tests;
