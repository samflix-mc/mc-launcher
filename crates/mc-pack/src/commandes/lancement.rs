//! Lancer le jeu sur l'instance installée.

mod annonce;
mod cible;
mod coherence;
mod instance;
mod partie;
mod preparation;

use anyhow::Result;
use mc_pack::source::Source;

use preparation::preparer;

pub async fn launch(
    source: &Source,
    options: &mc_pack::Options,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
    afficher: bool,
) -> Result<()> {
    let partie = preparer(source, options, pseudo, serveur, memoire).await?;
    annonce::annoncer(&partie);

    // Montrer sans lancer : c'est ce qu'on regarde quand le jeu refuse de
    // démarrer, et ce qui permet de rejouer la commande à la main.
    if afficher {
        println!("{}", partie.command.display());
        return Ok(());
    }
    partie::jouer(&partie).await
}
