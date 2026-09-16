//! Vérifier qu'une installation est complète et intacte.

use anyhow::Result;
use mc_pack::lockfile::Lockfile;
use mc_pack::source::Source;
use std::process::ExitCode;


pub fn verify(source: &Source, options: &mc_pack::Options, deep: bool) -> Result<ExitCode> {
    let problems = mc_pack::verify(source, options, deep)?;
    if problems.is_empty() {
        tracing::info!(
            exhaustif = deep,
            "Installation conforme au verrou{}",
            if deep { ", empreintes comprises" } else { "" }
        );
        println!("Installation complète et conforme au verrou.");
        return Ok(ExitCode::SUCCESS);
    }
    // Une anomalie de vérification n'est pas une panne du programme : elle
    // décrit l'installation. D'où `warn` plutôt que `error` — l'incident, c'est
    // le code de sortie, que l'appelant voit déjà.
    tracing::warn!(
        anomalies = problems.len(),
        exhaustif = deep,
        "Installation non conforme : {} anomalies",
        problems.len()
    );
    for problem in &problems {
        tracing::debug!(anomalie = %problem, "détail");
    }
    eprintln!("{} anomalies :", problems.len());
    for problem in &problems {
        eprintln!("  {problem}");
    }
    Ok(ExitCode::FAILURE)
}

/// Une dépendance introuvable n'empêche pas d'installer, mais empêchera le jeu
/// de démarrer : elle est signalée là où on la verra.
pub(super) fn report_unresolved(lock: &Lockfile) {
    if lock.unresolved.is_empty() {
        return;
    }
    eprintln!("\nDépendances introuvables :");
    for missing in &lock.unresolved {
        eprintln!(
            "  {} — exigé par {} ({})",
            missing.mod_id, missing.required_by, missing.side
        );
    }
    eprintln!(
        "  Ces mods manquent sur Modrinth comme sur CurseForge : le jeu refusera de démarrer."
    );
}
