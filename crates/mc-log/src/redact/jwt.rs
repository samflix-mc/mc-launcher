//! Les jetons qui se reconnaissent à leur seule forme.

use super::MASK;
use super::curseur::is_token_char;

/// Masque les jetons au format JWT.
///
/// Les jetons Microsoft et Minecraft en sont : trois segments base64url
/// séparés par des points, commençant par `eyJ` — soit `{"` encodé. Ce préfixe
/// suffit à les reconnaître sans se soucier du contexte, ce qui attrape aussi
/// les jetons qu'aucun mot-clé n'introduit.
pub(super) fn redact_jwt(text: &str) -> String {
    // Pas de jeton sans ce préfixe : l'écarter d'abord évite de recopier chaque
    // ligne du journal dans un `Vec<char>` pour n'y rien trouver.
    if !text.contains("eyJ") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i..].starts_with(&['e', 'y', 'J']) {
            let mut end = i;
            while end < chars.len() && is_token_char(chars[end]) {
                end += 1;
            }
            // Un identifiant qui commence par « eyJ » sans être un jeton est
            // trop court pour l'être : un JWT dépasse toujours largement.
            if end - i >= 24 {
                out.push_str(MASK);
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
#[path = "jwt.test.rs"]
mod tests;
