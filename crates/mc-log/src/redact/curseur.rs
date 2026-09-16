//! Où commence et où finit une valeur sensible dans un texte.

/// Un caractère peut-il appartenir à une valeur de jeton ?
///
/// Les jetons croisés ici sont du base64url, du JWT ou de l'hexadécimal, plus
/// les quelques ponctuations qu'un JWT contient. Tout le reste — espace,
/// guillemet, virgule, accolade — clôt la valeur.
pub(super) fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=' | '~' | '$')
}

/// Un caractère sépare-t-il un mot-clé de sa valeur ?
pub(super) fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, ':' | '=' | '"' | '\'' | ',')
}

/// Fin de la ponctuation qui commence à `from`.
pub(super) fn end_of_separators(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_separator(c)).unwrap_or(rest.len())
}

/// Fin de la valeur qui commence à `from`.
pub(super) fn end_of_value(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_token_char(c)).unwrap_or(rest.len())
}
