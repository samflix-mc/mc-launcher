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
    fn un_schema_d_authentification_autre_que_bearer_ne_laisse_pas_passer_la_valeur() {
        // « Basic » porte le couple identifiant/mot de passe en base64 : c'est
        // le contenu le plus sensible que cet en-tête puisse transporter. Le nom
        // du schéma, lui, doit survivre : c'est tout ce qui reste pour savoir de
        // quel en-tête il s'agissait.
        for (entree, attendu) in [
            (
                "Authorization: Basic dXNlcjpwYXNzd29yZA==",
                "Authorization: Basic [secret]",
            ),
            (
                "Authorization: Digest cnonce=abcdef123456",
                "Authorization: Digest [secret]",
            ),
            (
                "authorization: Token abcdef123456",
                "authorization: Token [secret]",
            ),
            // Valeur citée : sans ponctuation admise après le schéma, la ligne
            // ressortait intacte — sans même un masque pour le signaler.
            (
                "Authorization: Basic \"dXNlcjpwYXNzd29yZA==\"",
                "Authorization: Basic \"[secret]\"",
            ),
        ] {
            assert_eq!(redact(entree), attendu, "entrée : {entree}");
        }
    }

    #[test]
    fn un_secret_qui_commence_comme_un_schema_reste_masque_en_entier() {
        // Un schéma n'en est un que s'il forme un mot à part et qu'il introduit
        // quelque chose. Sinon il est la valeur, et la reconnaître déplaçait le
        // masque derrière les premiers caractères du secret.
        for (entree, attendu) in [
            ("token=basicSECRETVALUE", "token=[secret]"),
            ("password=dpop9f3a2b", "password=[secret]"),
            ("secret=token12345", "secret=[secret]"),
            ("api_key=bearerAAAA1111", "api_key=[secret]"),
            ("password: digest", "password: [secret]"),
            ("Authorization: Bearer", "Authorization: [secret]"),
        ] {
            assert_eq!(redact(entree), attendu, "entrée : {entree}");
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
