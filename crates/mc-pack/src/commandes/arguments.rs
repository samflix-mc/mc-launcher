//! La ligne de commande, telle que la page d'aide la décrit.

use anyhow::{Result, bail};
use std::path::PathBuf;

/// Ce que la ligne de commande a demandé.
///
/// Toutes les options sont lues avant que quoi que ce soit ne soit construit :
/// `--data` déplace la racine des données, dont dépend l'emplacement du cache
/// d'un pack distant.
#[derive(Debug)]
pub struct Arguments {
    pub command: String,
    pub source_arg: Option<String>,
    pub options: mc_pack::Options,
    pub deep: bool,
    pub incident_test: bool,
    pub pseudo: Option<String>,
    pub serveur: Option<String>,
    pub memoire: Option<u32>,
    pub afficher: bool,
}

impl Arguments {
    /// Rend `None` quand aucune commande n'est donnée : l'appelant affiche
    /// alors l'aide.
    pub fn lire(args: impl Iterator<Item = String>) -> Result<Option<Arguments>> {
        let args: Vec<String> = args.collect();
        let Some(command) = args.first().cloned() else {
            return Ok(None);
        };

        let mut lu = Arguments {
            command,
            source_arg: None,
            options: mc_pack::Options::default(),
            deep: false,
            incident_test: false,
            pseudo: None,
            serveur: None,
            memoire: None,
            afficher: false,
        };

        let mut rest = args[1..].iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--incident-test" => lu.incident_test = true,
                "--locked" => lu.options.locked = true,
                "--with-server" => lu.options.with_server = true,
                "--deep" => lu.deep = true,
                "--pseudo" => lu.pseudo = rest.next().cloned(),
                "--serveur" => lu.serveur = rest.next().cloned(),
                "--memoire" => lu.memoire = rest.next().and_then(|v| v.parse().ok()),
                "--afficher" => lu.afficher = true,
                "--instance" => lu.options.instance_name = rest.next().cloned(),
                "--data" => {
                    let Some(dir) = rest.next() else {
                        bail!("--data attend un répertoire");
                    };
                    lu.options.layout = mc_instance::Layout::new(PathBuf::from(dir));
                }
                other if other.starts_with("--") => bail!("option inconnue : {other}"),
                path => lu.source_arg = Some(path.to_string()),
            }
        }
        Ok(Some(lu))
    }
}

#[cfg(test)]
#[path = "arguments.test.rs"]
mod tests;
