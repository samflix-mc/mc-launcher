//! Censure des secrets avant qu'un texte ne quitte la machine.
//!
//! Ce launcher manipule des jetons d'accès Microsoft, des jetons Xbox Live, un
//! jeton Minecraft et une clé d'API CurseForge. Un rapport d'incident est un
//! texte libre : message d'erreur, chemin de fichier, URL, trace d'appels. Rien
//! n'empêche un jeton de s'y retrouver — une URL signée, un `Debug` de
//! structure, un message d'API qui répète la requête.
//!
//! Le filtre s'applique à **tout** ce qui part vers Sentry, événements comme
//! fils d'Ariane, et au fichier de journal. Il ne cherche pas à être malin : il
//! repère un petit nombre de formes connues et les remplace entièrement. Rendre
//! un incident un peu moins lisible est sans commune mesure avec la publication
//! d'un jeton qui donne accès à un compte Microsoft.

/// Ce qui remplace un secret.
const MASK: &str = "[secret]";

/// Mots qui annoncent une valeur sensible juste après.
const KEYWORDS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "device_code",
    "minecraft_token",
    "api_key",
    "apikey",
    "api-key",
    "x-api-key",
    "x-api-token",
    "authorization",
    "password",
    "secret",
    "token",
];

/// Schémas d'authentification HTTP : le mot annonce la valeur, il n'est pas la
/// valeur.
///
/// Sans cette liste, seul « Bearer » était reconnu. Les autres schémas étaient
/// pris pour le secret lui-même : c'est le nom du schéma qui se faisait masquer,
/// et l'identifiant qui le suit partait en clair.
///
/// Un schéma n'est reconnu que s'il forme un mot à part *et* s'il introduit
/// réellement quelque chose ; sinon c'est lui, la valeur. Sans ces deux
/// conditions, « token=basicXXXX » laissait passer « basic » en clair, et
/// « password: digest » ne masquait plus rien du tout.
const SCHEMES: &[&str] = &["bearer", "basic", "digest", "negotiate", "token", "dpop"];

/// Un caractère peut-il appartenir à une valeur de jeton ?
///
/// Les jetons croisés ici sont du base64url, du JWT ou de l'hexadécimal, plus
/// les quelques ponctuations qu'un JWT contient. Tout le reste — espace,
/// guillemet, virgule, accolade — clôt la valeur.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=' | '~' | '$')
}

/// Un caractère sépare-t-il un mot-clé de sa valeur ?
fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, ':' | '=' | '"' | '\'' | ',')
}

/// Fin de la ponctuation qui commence à `from`.
fn end_of_separators(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_separator(c)).unwrap_or(rest.len())
}

/// Fin de la valeur qui commence à `from`.
fn end_of_value(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_token_char(c)).unwrap_or(rest.len())
}

/// Remplace les secrets d'un texte.
pub fn redact(text: &str) -> String {
    let text = redact_jwt(text);
    let text = redact_after_keywords(&text);
    redact_home(&text)
}

/// Masque les jetons au format JWT.
///
/// Les jetons Microsoft et Minecraft en sont : trois segments base64url
/// séparés par des points, commençant par `eyJ` — soit `{"` encodé. Ce préfixe
/// suffit à les reconnaître sans se soucier du contexte, ce qui attrape aussi
/// les jetons qu'aucun mot-clé n'introduit.
fn redact_jwt(text: &str) -> String {
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

/// Masque ce qui suit un mot-clé sensible.
///
/// Couvre `token=abc`, `"access_token": "abc"`, `Authorization: Bearer abc` et
/// `--api-key abc` d'une seule règle : après le mot-clé, on saute les
/// séparateurs et la ponctuation d'usage, puis on efface la valeur.
fn redact_after_keywords(text: &str) -> String {
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

/// Remplace le répertoire personnel par `~`.
///
/// Un chemin absolu porte le nom de compte de l'utilisateur. Ce n'est pas un
/// secret, mais c'est une donnée personnelle qui n'apprend rien de plus que le
/// chemin relatif.
fn redact_home(text: &str) -> String {
    match HOME.as_deref() {
        Some(home) if text.contains(home) => text.replace(home, "~"),
        _ => text.to_string(),
    }
}

/// Le répertoire personnel, lu une seule fois.
///
/// `redact` voit passer chaque ligne écrite dans le journal : relire
/// l'environnement à chacune prenait son verrou global — que d'autres fils
/// écrivent par ailleurs — pour une valeur qui ne change pas de l'exécution.
/// La racine « / » est écartée : elle préfixe tout.
static HOME: std::sync::LazyLock<Option<String>> = std::sync::LazyLock::new(|| {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let home = home.to_string_lossy().into_owned();
    (!home.is_empty() && home != "/").then_some(home)
});

#[cfg(test)]
#[path = "redact.test.rs"]
mod tests;
