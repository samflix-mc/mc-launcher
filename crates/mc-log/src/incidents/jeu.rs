//! Le plantage du jeu, remonté comme un incident à part entière.

use crate::redact::redact;

/// Remonte un plantage du jeu comme un incident à part entière.
///
/// Minecraft plante dans sa propre JVM : rien n'en arrive au launcher, sinon
/// un code de sortie. Or c'est le seul moment où l'on dispose de la trace, et
/// un joueur ne pensera ni à la trouver ni à la joindre.
///
/// L'exception Java est donnée à Sentry comme une vraie exception, et non
/// comme un message : le type sert alors de clé de regroupement. Sans cela,
/// tous les plantages du jeu — quelle qu'en soit la cause — formeraient un
/// unique incident « Minecraft s'est arrêté », inexploitable.
///
/// Le texte passe par les mêmes filtres que le reste : un journal de jeu
/// contient le pseudo du joueur et des chemins absolus.
pub fn capture_game_crash(
    exception: &str,
    message: &str,
    excerpt: &str,
    context: &std::collections::BTreeMap<String, String>,
) -> sentry::types::Uuid {
    use sentry::protocol::{Event, Exception, Level, Value};

    let mut event = Event {
        level: Level::Error,
        // `logger` distingue d'emblée un plantage du jeu d'une erreur du
        // launcher, qui n'appellent pas le même travail.
        logger: Some("minecraft".into()),
        exception: vec![Exception {
            ty: exception.to_string(),
            value: Some(redact(message)),
            // Les modules Java ne sont pas des modules Sentry ; laisser le
            // champ vide évite un regroupement fantaisiste.
            module: None,
            ..Default::default()
        }]
        .into(),
        ..Default::default()
    };

    event.extra.insert(
        "journal".into(),
        Value::String(redact(&truncate(excerpt, 8_000))),
    );
    for (key, value) in context {
        event
            .extra
            .insert(key.clone(), Value::String(redact(value)));
    }

    // Pas de vidage ici : une partie produit jusqu'à cinq exceptions relevées
    // plus son plantage, et attendre la file à chaque fois immobilisait le
    // launcher dix secondes par exception dès que le réseau manquait. L'attente
    // se demande une fois, par l'appelant qui annonce l'identifiant — sinon par
    // [`Guard`] à la fermeture.
    sentry::capture_event(event)
}

/// Borne un texte sans couper au milieu d'une ligne.
fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    // La fin d'un journal est plus parlante que son début : c'est là que se
    // trouve ce qui a échoué.
    //
    // `max` est un nombre d'octets, et un journal de jeu en contient de
    // multi-octets — noms de mods accentués, « § », « … ». Trancher à l'aveugle
    // au milieu d'un caractère fait paniquer le découpage, et le launcher
    // mourrait dans la main qui rapporte le plantage.
    let mut start = text.len() - max;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let from = text[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(start);
    format!("[…début tronqué…]\n{}", &text[from..])
}

#[cfg(test)]
#[path = "jeu.test.rs"]
mod tests;
