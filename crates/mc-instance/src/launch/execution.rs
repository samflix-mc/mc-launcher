//! Lancer le jeu, et dire comment il s'est terminé.

use anyhow::{Context, Result};

mod compte_rendu;

use super::commande::Command;

pub use compte_rendu::{Outcome, Report};

/// Lance le jeu et attend qu'il se termine.
#[tracing::instrument(name = "exécution du jeu", skip_all)]
pub async fn run(command: &Command) -> Result<Report> {
    tracing::info!(
        java = %command.java.display(),
        repertoire = %command.working_dir.display(),
        arguments = command.args.len(),
        "Démarrage de Minecraft"
    );

    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // La sortie est captée pour être lue, puis réécrite telle quelle : le
    // joueur voit ce qu'il aurait vu, et le launcher peut en tirer les
    // exceptions au passage. Les deux flux sont fusionnés parce que Minecraft
    // écrit sur les deux sans distinction utile.
    let mut child = tokio::process::Command::new(&command.java)
        .args(&command.args)
        .current_dir(&command.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("exécution de {}", command.java.display()))?;

    let mut watcher = crate::crash::Watcher::new();
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    let mut out_lines = stdout.map(|r| r.lines());
    let mut err_lines = stderr.map(|r| r.lines());

    loop {
        let line = tokio::select! {
            Ok(Some(line)) = async {
                match &mut out_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            Ok(Some(line)) = async {
                match &mut err_lines {
                    Some(lines) => lines.next_line().await,
                    None => Ok(None),
                }
            } => Some(line),
            else => None,
        };
        match line {
            Some(line) => {
                println!("{line}");
                watcher.line(&line);
            }
            None => break,
        }
    }

    let status = child
        .wait()
        .await
        .with_context(|| format!("attente de {}", command.java.display()))?;

    let outcome = Outcome::from_status(&status);
    // Rien n'est journalisé en erreur ici : c'est l'appelant qui décide, et
    // c'est lui qui connaît le contexte. Le faire aux deux endroits produisait
    // deux incidents distincts pour un seul échec, constaté sur MC-LAUNCHER-4
    // et MC-LAUNCHER-5.
    tracing::debug!(?outcome, code = status.code(), "Minecraft s'est terminé");

    Ok(Report {
        outcome,
        errors: watcher.finish(),
    })
}

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
