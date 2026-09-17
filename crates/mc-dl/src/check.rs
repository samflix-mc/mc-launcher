//! Jusqu'où vérifier un fichier déjà présent.

use crate::Checksum;
use std::path::Path;

/// Ce qu'a fait [`Downloader::to_file`], pour distinguer un vrai
/// téléchargement d'un fichier déjà conforme dans le compte rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetched {
    Downloaded,
    AlreadyPresent,
}

/// Comment décider qu'un fichier déjà présent n'a pas besoin d'être repris.
///
/// La distinction n'est pas cosmétique : les assets d'une version de Minecraft
/// pèsent plus de 800 Mo répartis sur quelques milliers d'objets. Recalculer
/// leur SHA-1 à chaque lancement relit tout le disque pour ne presque jamais
/// rien trouver.
#[derive(Debug, Clone, Copy)]
pub enum Check<'a> {
    /// Empreinte recalculée à chaque passage. Pour ce qui est exécuté — jars,
    /// bibliothèques, runtimes.
    Full(&'a Checksum),
    /// Taille comme première barrière, empreinte vérifiée seulement à
    /// l'écriture. Pour les gros volumes de petits fichiers inertes : un asset
    /// tronqué a la mauvaise taille, et une altération silencieuse à taille
    /// constante donne au pire une texture fausse, jamais du code exécuté.
    /// La vérification exhaustive reste disponible à la demande.
    Quick { sum: &'a Checksum, size: u64 },
    /// Aucune empreinte publiée, mais une taille annoncée. C'est tout ce
    /// qu'offrent certaines sources ; mieux vaut contrôler la taille que rien,
    /// une réponse d'erreur servie en HTTP 200 ne faisant jamais le bon
    /// nombre d'octets.
    Size(u64),
    /// Aucune empreinte publiée : seule la présence peut être constatée.
    Presence,
}

impl Check<'_> {
    pub(crate) fn checksum(&self) -> Option<&Checksum> {
        match self {
            Check::Full(sum) => Some(sum),
            Check::Quick { sum, .. } => Some(sum),
            Check::Size(_) | Check::Presence => None,
        }
    }

    /// Taille attendue, quand la source la publie.
    pub(crate) fn size(&self) -> Option<u64> {
        match self {
            Check::Quick { size, .. } | Check::Size(size) => Some(*size),
            Check::Full(_) | Check::Presence => None,
        }
    }

    /// Le fichier présent peut-il être conservé sans téléchargement ?
    pub(crate) fn accepts_existing(&self, path: &Path) -> bool {
        match self {
            Check::Full(sum) => std::fs::read(path)
                .map(|b| sum.matches(&b))
                .unwrap_or(false),
            Check::Quick { size, .. } | Check::Size(size) => std::fs::metadata(path)
                .map(|m| m.len() == *size)
                .unwrap_or(false),
            Check::Presence => true,
        }
    }
}

#[cfg(test)]
#[path = "check.test.rs"]
mod tests;
