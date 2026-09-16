//! Où vit la session entre deux lancements.
//!
//! Le fichier contient un jeton de rafraîchissement : quiconque le lit peut
//! rouvrir la session du joueur sans mot de passe et sans second facteur. Il
//! est donc écrit en `0600`, et jamais journalisé — la censure de `mc-log`
//! reconnaît déjà `refresh_token` et `access_token`, mais le mieux reste de ne
//! pas l'écrire.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Emplacement du fichier de session.
///
/// À côté de la clé CurseForge, dans la configuration et non dans les données :
/// c'est un secret de l'utilisateur, pas un cache reconstructible.
pub fn chemin() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("samflix-mc").join("session.json")
}

/// La session enregistrée, ou `None` si personne ne s'est connecté ici.
///
/// Un fichier illisible n'est pas une erreur fatale : il vaut « pas de
/// session », et l'appelant proposera de se connecter. Le contraire
/// empêcherait de jouer à cause d'un fichier corrompu.
pub fn charger() -> Option<serde_json::Value> {
    let brut = std::fs::read(chemin()).ok()?;
    match serde_json::from_slice(&brut) {
        Ok(etat) => Some(etat),
        Err(erreur) => {
            tracing::warn!(
                fichier = %chemin().display(),
                erreur = %erreur,
                "session enregistrée illisible, connexion à refaire"
            );
            None
        }
    }
}

/// Écrit la session, lisible par son seul propriétaire.
pub fn enregistrer(etat: &serde_json::Value) -> Result<()> {
    let chemin = chemin();
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création de {}", parent.display()))?;
    }

    let brut = serde_json::to_vec_pretty(etat).context("sérialisation de la session")?;
    ecrire_protege(&chemin, &brut)
        .with_context(|| format!("écriture de {}", chemin.display()))?;

    tracing::debug!(fichier = %chemin.display(), "session enregistrée");
    Ok(())
}

/// Oublie la session. Ne pas en avoir n'est pas une erreur.
pub fn effacer() -> Result<()> {
    match std::fs::remove_file(chemin()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("suppression de {}", chemin().display())),
    }
}

/// Crée le fichier en `0600` **avant** d'y écrire.
///
/// Écrire puis restreindre laisserait une fenêtre pendant laquelle le jeton
/// est lisible par tous ; sur un poste partagé, cette fenêtre suffit.
#[cfg(unix)]
fn ecrire_protege(chemin: &std::path::Path, contenu: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut fichier = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(chemin)?;
    fichier.write_all(contenu)
}

/// Windows n'a pas de bit de permission équivalent : le fichier hérite des
/// droits du répertoire, qui est déjà sous le profil de l'utilisateur.
#[cfg(not(unix))]
fn ecrire_protege(chemin: &std::path::Path, contenu: &[u8]) -> std::io::Result<()> {
    std::fs::write(chemin, contenu)
}

#[cfg(test)]
#[path = "stockage.test.rs"]
mod tests;
