//! De la ligne de commande à la commande qui s'exécute.

use anyhow::Result;
use std::process::ExitCode;

use mc_pack::source::{self, Source};

use crate::commandes::arguments::Arguments;
use crate::commandes::{self, diagnostic, usage};
use crate::journal;

pub async fn run(log: &mc_log::Guard) -> Result<ExitCode> {
    let Some(args) = Arguments::lire(std::env::args().skip(1))? else {
        usage();
        return Ok(ExitCode::from(2));
    };
    let Arguments {
        command,
        source_arg,
        options,
        deep,
        incident_test,
        pseudo,
        serveur,
        memoire,
        afficher,
    } = args;

    // Le diagnostic n'a pas besoin de manifeste : c'est justement ce qu'on
    // lance quand on ne sait pas encore ce qui va de travers.
    if command == "diagnostic" {
        return diagnostic(log, incident_test);
    }

    // La source est construite après la lecture des arguments : `--data` peut
    // déplacer la racine des données, dont dépend le cache d'un pack distant.
    let source = Source::parse(
        source_arg.as_deref().unwrap_or(source::url_par_defaut()),
        &options.layout,
    );

    let _span = journal::ouvrir(&command, &source);
    let debut = std::time::Instant::now();

    let result = commandes::executer(
        &command, &source, &options, deep, pseudo, serveur, memoire, afficher,
    )
    .await;

    journal::conclure(&command, &result, debut, log);
    result
}
