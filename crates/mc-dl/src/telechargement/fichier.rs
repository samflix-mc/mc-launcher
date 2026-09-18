//! Ce qui arrive sur le disque, et comment il y arrive.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::progression::Avancement;
use crate::{Check, Downloader, Fetched};

impl Downloader {
    /// Télécharge `url` vers `dest`, sauf si `dest` satisfait déjà `check`.
    ///
    /// Ce qui est écrit est **toujours** vérifié quand une empreinte existe ;
    /// `check` ne règle que la sévérité du contrôle sur un fichier déjà là.
    ///
    /// C'est ici que le nom du fichier est connu, donc ici qu'il est annoncé :
    /// [`Downloader::bytes`] ne voit qu'une URL, et une URL de CDN ne dit rien
    /// à personne.
    pub async fn to_file(&self, url: &str, dest: &Path, check: Check<'_>) -> Result<Fetched> {
        let nom = nom_court(dest);
        self.signaler(Avancement::Debut {
            fichier: &nom,
            octets: check.size(),
        });

        if dest.is_file() && check.accepts_existing(dest) {
            tracing::trace!(fichier = %dest.display(), "déjà conforme");
            // La taille vient du disque plutôt que de `check` : `Check::Full`
            // n'en publie pas, et sans elle un fichier déjà là ne ferait pas
            // avancer la barre — une réinstallation resterait à zéro de bout
            // en bout.
            //
            // `std::fs::metadata` et NON `tokio::fs::metadata`, dans une
            // fonction pourtant `async`, et c'est délibéré. Ce chemin est
            // celui d'un fichier DÉJÀ conforme : il s'exécute plusieurs
            // milliers de fois à chaque vérification — un `stat` par objet
            // d'assets. Un `stat` coûte quelques microsecondes ; l'envoyer sur
            // le pool de `spawn_blocking` coûterait la création et
            // l'ordonnancement d'une tâche, soit davantage que l'appel
            // lui-même, et multiplié par trois mille.
            //
            // La règle générique — « pas d'appel bloquant dans une fonction
            // asynchrone » — vise les opérations dont la durée est
            // imprévisible : lire ou écrire un contenu. C'est pour cela que
            // l'ÉCRITURE, juste en dessous, passe par `ecrire_hors_du_fil`.
            self.signaler(Avancement::Fini {
                fichier: &nom,
                etat: Fetched::AlreadyPresent,
                octets: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0),
            });
            return Ok(Fetched::AlreadyPresent);
        }

        let bytes = self.bytes(url).await?;
        if let Some(sum) = check.checksum() {
            sum.verify(&bytes, &dest.display().to_string())?;
        } else if let Some(expected) = check.size() {
            // Faute d'empreinte, la taille est le seul contrôle possible. Il
            // suffit à écarter une page d'erreur ou une redirection servie en
            // HTTP 200, qui est le cas de loin le plus fréquent.
            if bytes.len() as u64 != expected {
                bail!(
                    "{} : {} octets reçus, {expected} annoncés",
                    dest.display(),
                    bytes.len()
                );
            }
        }
        // La longueur AVANT de céder les octets : la tâche détachée les
        // possède, et il n'y a plus rien à mesurer après.
        let poids = bytes.len();
        ecrire_hors_du_fil(dest, bytes).await?;
        tracing::debug!(
            fichier = %dest.display(),
            octets = poids,
            verifie = check.checksum().is_some(),
            "téléchargé"
        );
        self.signaler(Avancement::Fini {
            fichier: &nom,
            etat: Fetched::Downloaded,
            octets: poids as u64,
        });
        Ok(Fetched::Downloaded)
    }
}

/// Le nom du fichier seul, tel qu'on l'affiche.
///
/// Le chemin complet déborderait de la fenêtre et n'apprendrait rien : entre
/// `~/.local/share/samflix-mc/shared/assets/objects/a3/a3f1…` et `a3f1…`, seul
/// le second tient sur une ligne. À défaut de nom — un chemin qui finit par
/// `..` —, le chemin complet vaut mieux que rien.
fn nom_court(dest: &Path) -> String {
    dest.file_name()
        .unwrap_or(dest.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Écrit par `.part` puis renomme : sur le même système de fichiers, le
/// renommage est atomique, donc `dest` n'existe qu'une fois complet.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création de {}", parent.display()))?;
    }
    let part: PathBuf = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    std::fs::write(&part, bytes).with_context(|| format!("écriture de {}", part.display()))?;
    std::fs::rename(&part, dest).with_context(|| format!("renommage vers {}", dest.display()))?;
    Ok(())
}

/// La même écriture, mais **hors du fil d'exécution asynchrone**.
///
/// `write_atomic` bloque tant que le disque n'a pas répondu. Appelée depuis une
/// fonction `async`, elle bloque non pas la tâche mais le WORKER de tokio :
/// pendant qu'un jar de cinquante mégaoctets s'écrit, ce fil n'avance aucune
/// autre tâche — et les téléchargements, qui sont lancés concurremment, se
/// sérialisent derrière le plus lent d'entre eux. Sur une première
/// installation, cela se compte en minutes.
///
/// Le pool de `spawn_blocking` est séparé de celui des tâches : c'est
/// exactement ce pour quoi il existe.
///
/// Prend les octets par valeur plutôt que par référence : la tâche détachée
/// doit posséder ce qu'elle écrit, et l'appelant n'en a plus besoin — il en
/// connaissait déjà la longueur.
pub async fn ecrire_hors_du_fil(dest: &Path, octets: Vec<u8>) -> Result<()> {
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || write_atomic(&dest, &octets))
        .await
        .context("l'écriture détachée n'a pas abouti")?
}

/// Lire un fichier, hors du fil d'exécution asynchrone.
///
/// Le pendant d'[`ecrire_hors_du_fil`], et pour la même raison : une lecture
/// bloque le worker de tokio, pas seulement la tâche. Elle vit ici plutôt que
/// chez ses appelants pour que `mc-nouvelles` — dont la moitié pure n'a aucune
/// dépendance asynchrone — n'ait pas à gagner tokio pour une ligne.
pub async fn lire_hors_du_fil(source: &Path) -> std::io::Result<Vec<u8>> {
    let source = source.to_path_buf();
    match tokio::task::spawn_blocking(move || std::fs::read(source)).await {
        Ok(resultat) => resultat,
        Err(erreur) => Err(std::io::Error::other(erreur)),
    }
}

#[cfg(test)]
#[path = "fichier.test.rs"]
mod tests;
