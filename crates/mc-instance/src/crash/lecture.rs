//! Ce qu'est un plantage, et comment on l'extrait d'un texte.

mod analyse;

use std::path::PathBuf;

pub(super) use analyse::{split_exception, strip_ansi};


/// Ce qu'on a pu apprendre d'un arrêt anormal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crash {
    /// Type de l'exception Java, p. ex. `java.lang.module.ResolutionException`.
    ///
    /// C'est lui qui sert de titre : sans ça, tous les plantages du jeu se
    /// regrouperaient en un seul incident indistinct.
    pub exception: String,
    /// Message porté par l'exception.
    pub message: String,
    /// Extrait du fichier, borné pour rester lisible et envoyable.
    pub excerpt: String,
    /// Fichier d'où vient l'information.
    pub source: PathBuf,
}

/// Nombre de lignes conservées autour de l'exception.
///
/// Assez pour la trace et le contexte immédiat, pas assez pour dépasser les
/// limites d'un événement Sentry ni pour être illisible.
pub(super) const EXCERPT_LINES: usize = 60;

/// Cherche de quoi expliquer un arrêt anormal.
///
/// `started_at` écarte les rapports d'une partie précédente : un crash vieux

/// (`Caused by`, exceptions de fermeture), et c'est celle d'origine qui
/// identifie le problème.
pub fn parse(text: &str) -> Option<Crash> {
    let lines: Vec<&str> = text.lines().collect();

    let (index, exception, message) = lines.iter().enumerate().find_map(|(i, line)| {
        let (exception, message) = split_exception(line)?;
        Some((i, exception, message))
    })?;

    // L'extrait démarre un peu avant : les lignes qui précèdent disent souvent
    // ce que le jeu était en train de faire.
    let start = index.saturating_sub(5);
    let end = (index + EXCERPT_LINES).min(lines.len());
    let excerpt = lines[start..end].join("\n");

    Some(Crash {
        exception,
        message,
        excerpt: strip_ansi(&excerpt),
        source: PathBuf::new(),
    })
}

/// Reconnaît `paquet.Classe: message` dans une ligne.
///
/// Une trace Java nomme sa classe par un chemin pointé finissant par un
/// identifiant capitalisé. Exiger les deux évite de prendre pour une exception
/// un horodatage ou un chemin de fichier, qui contiennent aussi des points et
/// des deux-points.

#[cfg(test)]
#[path = "lecture.test.rs"]
mod tests;
