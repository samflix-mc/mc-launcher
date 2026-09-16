//! Ce qu'un mot-clé annonce.

use super::curseur::{end_of_separators, end_of_value};
use super::tables::{KEYWORDS, SCHEMES};
use super::MASK;

/// Masque ce qui suit un mot-clé sensible.
///
/// Couvre `token=abc`, `"access_token": "abc"`, `Authorization: Bearer abc` et
/// `--api-key abc` d'une seule règle : après le mot-clé, on saute les
/// séparateurs et la ponctuation d'usage, puis on efface la valeur.
pub(super) fn redact_after_keywords(text: &str) -> String {
    // Les mots-clés sont en ASCII pur et `to_ascii_lowercase` ne change aucune
    // longueur : les indices d'octets de `lower` sont exactement ceux de `text`.
    // Comparer les deux `&str` directement évite les deux `Vec<char>` de la
    // ligne entière, plus les vingt que découper les tables coûtait — et cette
    // fonction voit passer chaque ligne écrite dans le journal.
    let lower = text.to_ascii_lowercase();
    // Le cas courant, et de très loin : une ligne de journal sur mille porte un
    // mot-clé. Les autres n'ont rien à recopier.
    if !KEYWORDS.iter().any(|keyword| lower.contains(keyword)) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    'outer: while i < text.len() {
        for keyword in KEYWORDS {
            if !lower[i..].starts_with(keyword) {
                continue;
            }
            // Séparateurs entre le mot-clé et sa valeur.
            let mut j = end_of_separators(text, i + keyword.len());

            // Le schéma annonce la valeur : il reste lisible et l'on continue
            // jusqu'à ce qu'il introduit. À deux conditions, sans quoi le masque
            // se déplaçait au mauvais endroit : qu'il forme un mot à part, et
            // qu'il introduise réellement quelque chose.
            for scheme in SCHEMES {
                if !lower[j..].starts_with(scheme) {
                    continue;
                }
                let after = end_of_separators(text, j + scheme.len());
                if after > j + scheme.len() && end_of_value(text, after) > after {
                    j = after;
                }
                break;
            }

            // Le mot-clé et ce qui l'accompagne restent lisibles : sans eux, on
            // ne saurait pas de quel secret il s'agissait.
            out.push_str(&text[i..j]);
            let end = end_of_value(text, j);
            if end > j {
                out.push_str(MASK);
            }
            i = end;
            continue 'outer;
        }
        let Some(c) = text[i..].chars().next() else {
            break;
        };
        out.push(c);
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
#[path = "mot_cle.test.rs"]
mod tests;
