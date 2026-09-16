//! Reconnaître une exception Java dans une ligne de journal.

/// Reconnaît `paquet.Classe: message` dans une ligne.
///
/// Une trace Java nomme sa classe par un chemin pointé finissant par un
/// identifiant capitalisé. Exiger les deux évite de prendre pour une exception
/// un horodatage ou un chemin de fichier, qui contiennent aussi des points et
/// des deux-points.
pub(crate) fn split_exception(line: &str) -> Option<(String, String)> {
    let line = strip_ansi(line);
    let line = line.trim();
    // Les lignes de trace commencent par « at » : ce sont des cadres, pas la
    // déclaration de l'exception.
    if line.starts_with("at ") {
        return None;
    }

    let candidate = match line.find("Caused by: ") {
        Some(pos) => &line[pos + "Caused by: ".len()..],
        None => line,
    };
    // Une ligne de journal préfixée d'un horodatage et d'une catégorie : on ne
    // garde que ce qui suit le dernier « ]: ».
    let candidate = match candidate.rfind("]: ") {
        Some(pos) => &candidate[pos + 3..],
        None => candidate,
    };

    let (class, message) = candidate.split_once(':')?;
    let class = class.trim();
    if !class.contains('.') || class.contains(' ') || class.contains('/') {
        return None;
    }
    let last = class.rsplit('.').next()?;
    if !last.chars().next()?.is_ascii_uppercase() {
        return None;
    }
    // Filet supplémentaire : une classe d'exception se termine presque
    // toujours ainsi, et s'en tenir là écarte les faux positifs restants.
    if !(last.ends_with("Exception") || last.ends_with("Error") || last.ends_with("Throwable")) {
        return None;
    }

    Some((class.to_string(), message.trim().to_string()))
}

/// Retire les séquences de couleur ANSI.
///
/// Minecraft colore sa sortie ; conservées, ces séquences rendent l'extrait
/// illisible dans un rapport d'incident.
pub(crate) fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "analyse.test.rs"]
mod tests;
