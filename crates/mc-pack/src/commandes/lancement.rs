//! Lancer le jeu sur l'instance installée.
//!
//! La préparation et l'exécution sont dans la bibliothèque — `mc_pack::jeu` —
//! parce que la fenêtre les appelle aussi. Ce qui reste ici est ce qui
//! appartient à un terminal : le récapitulatif avant de lancer, et ce qu'on
//! écrit quand la partie s'achève.

mod annonce;

use anyhow::{Result, bail};
use mc_pack::source::Source;
use mc_pack::{Identite, jeu};

/// Hors de portée des tests de mutation : cette commande prépare la partie —
/// session, Java, instance — puis lance Minecraft. Ce qu'elle assemble est
/// vérifié pièce par pièce dans `mc_pack::jeu` ; ce qu'elle affiche l'est par
/// `annonce` et [`lignes_d_erreurs`].
#[mutants::skip]
pub async fn launch(
    source: &Source,
    options: &mc_pack::Options,
    pseudo: Option<String>,
    serveur: Option<String>,
    memoire: Option<u32>,
    afficher: bool,
) -> Result<()> {
    // `--pseudo` demande une session hors-ligne, son absence demande le compte
    // enregistré. Le choix est explicite des deux côtés : un repli silencieux
    // ferait entrer un joueur sur un serveur sous une identité qu'il n'a pas
    // choisie.
    let identite = match pseudo {
        Some(pseudo) => Identite::HorsLigne(pseudo),
        None => Identite::Microsoft,
    };

    let partie = jeu::preparer(source, options, identite, serveur, memoire).await?;
    annonce::annoncer(&partie);

    // Montrer sans lancer : c'est ce qu'on regarde quand le jeu refuse de
    // démarrer, et ce qui permet de rejouer la commande à la main.
    if afficher {
        println!("{}", partie.command.display());
        return Ok(());
    }

    let report = jeu::jouer(&partie).await?;
    for ligne in lignes_d_erreurs(&report.errors) {
        eprintln!("{ligne}");
    }

    match report.outcome {
        mc_instance::launch::Outcome::Normal => Ok(()),
        mc_instance::launch::Outcome::Interrupted { signal } => {
            println!("Minecraft a été interrompu (signal {signal}).");
            Ok(())
        }
        // Le seul cas qui rend un code de sortie non nul : c'est ici que la
        // ligne de commande décide, pas la bibliothèque.
        mc_instance::launch::Outcome::Failed { code } => bail!(
            "Minecraft s'est arrêté avec le code {code} — journaux dans {}",
            jeu::journaux(&partie).display()
        ),
    }
}

/// Ce qu'on dit au joueur des erreurs relevées pendant sa partie.
///
/// Vide quand il n'y en a pas : annoncer « 0 erreurs relevées » après une
/// partie qui s'est bien passée ferait chercher une panne inexistante. Sinon,
/// chacune est nommée — Minecraft en rattrape beaucoup et continue, et ce sont
/// souvent elles qui expliquent un comportement signalé bien plus tard.
fn lignes_d_erreurs(erreurs: &[mc_instance::crash::Crash]) -> Vec<String> {
    if erreurs.is_empty() {
        return Vec::new();
    }
    let mut lignes = vec![format!(
        "\n{} erreurs relevées pendant la partie :",
        erreurs.len()
    )];
    lignes.extend(
        erreurs
            .iter()
            .map(|erreur| format!("  {} : {}", erreur.exception, erreur.message)),
    );
    lignes
}

#[cfg(test)]
#[path = "lancement.test.rs"]
mod tests;
