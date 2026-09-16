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

/// Un caractère peut-il appartenir à une valeur de jeton ?
///
/// Les jetons croisés ici sont du base64url, du JWT ou de l'hexadécimal, plus
/// les quelques ponctuations qu'un JWT contient. Tout le reste — espace,
/// guillemet, virgule, accolade — clôt la valeur.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=' | '~' | '$')
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
    let lower = text.to_ascii_lowercase();
    let bytes: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    'outer: while i < bytes.len() {
        for keyword in KEYWORDS {
            let kw: Vec<char> = keyword.chars().collect();
            if lower_chars[i..].starts_with(&kw[..]) {
                // Le mot-clé lui-même reste lisible : sans lui, on ne saurait
                // pas de quel secret il s'agissait.
                out.extend(&bytes[i..i + kw.len()]);
                let mut j = i + kw.len();

                // Séparateurs entre le mot-clé et sa valeur, « Bearer » compris.
                let mut separator = String::new();
                while j < bytes.len()
                    && (bytes[j].is_whitespace()
                        || matches!(bytes[j], ':' | '=' | '"' | '\'' | ','))
                {
                    separator.push(bytes[j]);
                    j += 1;
                }
                if lower_chars[j..].starts_with(&['b', 'e', 'a', 'r', 'e', 'r']) {
                    separator.extend(&bytes[j..j + 6]);
                    j += 6;
                    while j < bytes.len() && bytes[j].is_whitespace() {
                        separator.push(bytes[j]);
                        j += 1;
                    }
                }
                out.push_str(&separator);

                let start = j;
                while j < bytes.len() && is_token_char(bytes[j]) {
                    j += 1;
                }
                if j > start {
                    out.push_str(MASK);
                }
                i = j;
                continue 'outer;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Remplace le répertoire personnel par `~`.
///
/// Un chemin absolu porte le nom de compte de l'utilisateur. Ce n'est pas un
/// secret, mais c'est une donnée personnelle qui n'apprend rien de plus que le
/// chemin relatif.
fn redact_home(text: &str) -> String {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return text.to_string();
    };
    let home = home.to_string_lossy().to_string();
    if home.is_empty() || home == "/" {
        return text.to_string();
    }
    text.replace(&home, "~")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_jwt_est_masque_meme_sans_mot_cle() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk";
        let sortie = redact(&format!("échec avec {jwt} en tête"));
        assert!(!sortie.contains("eyJhbGci"));
        assert!(sortie.contains(MASK));
        assert!(sortie.contains("échec avec"));
    }

    #[test]
    fn les_formes_usuelles_de_jeton_sont_couvertes() {
        for entree in [
            "access_token=ya29.A0ARrdaM9xQ",
            "\"refresh_token\": \"M.C123_BAY.0.U.ArFbK\"",
            "Authorization: Bearer abcdef123456",
            "x-api-key: $2a$10$abcdefghijklmnop",
            // UUID inventé : un secret réel n'a rien à faire dans un test,
            // il finirait versionné pour toujours.
            "--api-key 00000000-1111-2222-3333-444444444444",
        ] {
            let sortie = redact(entree);
            assert!(sortie.contains(MASK), "non masqué : {entree} → {sortie}");
        }
    }

    #[test]
    fn le_nom_du_champ_reste_lisible() {
        // Sans le mot-clé, un incident ne dirait plus de quel jeton il s'agit.
        let sortie = redact("refresh_token=M.C123_BAY");
        assert!(sortie.starts_with("refresh_token="));
        assert!(sortie.ends_with(MASK));
    }

    #[test]
    fn un_texte_sans_secret_est_intact() {
        let texte = "téléchargement de jei-1.21.1-neoforge-19.51.0.418.jar (1.7 Mio)";
        assert_eq!(redact(texte), texte);
    }

    #[test]
    fn une_empreinte_n_est_pas_prise_pour_un_secret() {
        // Les SHA-1 sont utiles au diagnostic et ne révèlent rien.
        let texte = "empreinte 88ee316e68900080b017f60c12162e2731924cf8 attendue";
        assert_eq!(redact(texte), texte);
    }

    #[test]
    fn un_identifiant_court_commencant_par_ey_survit() {
        // « eyZ2YBGT » est un identifiant de version Modrinth, pas un jeton.
        let texte = "build épinglé eyZ2YBGT introuvable";
        assert_eq!(redact(texte), texte);
    }

    #[test]
    fn le_repertoire_personnel_devient_un_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }
        let sortie = redact(&format!("{home}/.local/share/samflix-mc/logs"));
        assert!(sortie.starts_with('~'));
        assert!(!sortie.contains(&home));
    }

    #[test]
    fn plusieurs_secrets_dans_un_meme_texte() {
        let sortie = redact("token=abc123456 puis api_key=def789012 fin");
        assert_eq!(sortie.matches(MASK).count(), 2);
        assert!(sortie.contains("puis"));
        assert!(sortie.ends_with("fin"));
    }
}
