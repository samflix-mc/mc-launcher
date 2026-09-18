//! Quelles URL un billet a le droit de porter.
//!
//! ## Deux règles, et deux dangers différents
//!
//! Les **images** doivent venir du même hôte que le fil. Elles sont
//! téléchargées par Rust et affichées dans la fenêtre : une image d'ailleurs
//! ferait joindre un tiers à chaque ouverture de la page.
//!
//! Les **liens** ont le droit de sortir : c'est le propre d'un lien. Ils ne
//! sont jamais suivis dans la fenêtre — le front les confie à `ouvrirPage()`,
//! qui les donne au navigateur du système, et le greffon de navigation
//! refuserait de toute façon. Ce qu'on filtre chez eux est donc le SCHÉMA :
//! `javascript:` et `data:` n'ont aucun sens dans un billet, et tout à gagner
//! à être refusés.

/// Le seul endroit qui décide de ce qu'est « le même hôte ».
///
/// ## Le piège du préfixe
///
/// Comparer par `starts_with` sur l'URL entière est faux, et faux d'une façon
/// qui se démontre : `https://mc-launcher.ggy.info.evil.example/` commence
/// bien par `https://mc-launcher.ggy.info`. On compare donc l'hôte EXTRAIT, en
/// entier, et le port avec.
///
/// ## Pourquoi le schéma n'est pas figé à `https:`
///
/// `mc_essais::Serveur` ne sert que du `http://127.0.0.1:<port>`. Figer
/// `https:` rendrait toute la moitié réseau de ce crate intestable — et un
/// module qu'on ne peut pas éprouver finit par contenir ce qu'on n'a pas voulu.
/// La règle est donc « même schéma ET même hôte que le fil », ce qui est plus
/// strict en production, où le fil est en `https:`.
pub fn meme_origine(url: &str, hote_du_fil: &str) -> bool {
    match (origine(url), origine(hote_du_fil)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// `schéma://hôte[:port]`, sans le chemin.
fn origine(url: &str) -> Option<String> {
    let (schema, reste) = url.split_once("://")?;
    if schema.is_empty() {
        return None;
    }
    let autorite = reste.split(['/', '?', '#']).next()?;
    if autorite.is_empty() {
        return None;
    }
    // Un `user@host` : ce qui compte est après l'arobase. Sans ce découpage,
    // `https://mc-launcher.ggy.info@evil.example/` passerait pour notre hôte.
    let hote = autorite.rsplit('@').next()?;
    if hote.is_empty() {
        return None;
    }
    Some(format!(
        "{}://{}",
        schema.to_ascii_lowercase(),
        hote.to_ascii_lowercase()
    ))
}

/// L'URL absolue d'une image de billet, ou `None` si elle sort de l'hôte.
///
/// `image` peut être relative au fil — c'est la forme attendue — ou absolue,
/// auquel cas elle doit être sur le même hôte.
pub fn image_absolue(image: &str, url_du_fil: &str) -> Option<String> {
    if image.contains("://") {
        return meme_origine(image, url_du_fil).then(|| image.to_string());
    }
    // Relative : on la résout contre le répertoire du fil.
    let base = url_du_fil.rsplit_once('/')?.0;
    let propre = image.trim_start_matches('/');
    // Un `..` remonterait hors du répertoire publié. On refuse plutôt que de
    // normaliser : rien de légitime n'en a besoin.
    if propre.split('/').any(|part| part == "..") {
        return None;
    }
    Some(format!("{base}/{propre}"))
}

/// Les schémas qu'un lien de billet a le droit de porter.
const SCHEMAS_DE_LIEN: [&str; 3] = ["https://", "http://", "mailto:"];

/// Une URL de lien acceptable, normalisée.
///
/// Rend `None` pour tout ce qui n'est pas un des schémas ci-dessus :
/// `javascript:`, `data:`, `file:` n'ont aucun sens dans un billet.
pub fn acceptable(href: &str, _hote_du_fil: &str) -> Option<String> {
    let nu = href.trim();
    // La comparaison est insensible à la casse : `JavaScript:` passerait
    // sinon entre les mailles.
    let minuscules = nu.to_ascii_lowercase();
    SCHEMAS_DE_LIEN
        .iter()
        .any(|schema| minuscules.starts_with(schema))
        .then(|| nu.to_string())
}

#[cfg(test)]
#[path = "liens.test.rs"]
mod tests;
