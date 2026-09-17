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
mod commandes;
mod deroulement;
mod journal;

use std::process::ExitCode;

use deroulement::run;

#[tokio::main]
async fn main() -> ExitCode {
    // Avant tout le reste : une erreur de lecture d'arguments mérite déjà
    // d'être journalisée, et le guard doit vivre jusqu'à la fin du programme
    // pour que le journal et les incidents partent complètement.
    //
    // Il vit jusqu'au retour de `main`, et pas une instruction de moins : les
    // destructeurs tournent avant que le processus ne rende son code. C'est ce
    // que les `std::process::exit` semés dans les commandes empêchaient — un
    // `verify` non conforme journalisait ses anomalies, puis coupait le
    // programme avant que la file d'écriture ne les ait posées dans le fichier
    // qu'il venait lui-même de citer. D'où le code de sortie rendu, jamais pris.
    let _log = mc_log::init("mc-pack");

    match run(&_log).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Erreur : {error:?}");
            ExitCode::FAILURE
        }
    }
}
