use super::niveau_de;

/// Les cinq noms que le front emploie, et ce qu'ils valent.
///
/// Écrits des deux côtés : `noyau/journal.ts` n'en connaît pas d'autres, et un
/// nom qui divergerait ferait retomber toutes les lignes d'un niveau sur
/// `info` — c'est-à-dire un journal qui ne distingue plus rien, sans erreur.
#[test]
fn les_cinq_niveaux_sont_reconnus() {
    assert_eq!(niveau_de("error"), tracing::Level::ERROR);
    assert_eq!(niveau_de("warn"), tracing::Level::WARN);
    assert_eq!(niveau_de("info"), tracing::Level::INFO);
    assert_eq!(niveau_de("debug"), tracing::Level::DEBUG);
    assert_eq!(niveau_de("trace"), tracing::Level::TRACE);
}

/// **Un niveau inconnu ne perd pas la ligne.**
///
/// Le contraire — écarter ce qu'on ne sait pas nommer — ferait disparaître
/// silencieusement un message de diagnostic à cause d'une faute de frappe, au
/// moment précis où l'on cherche pourquoi quelque chose manque.
#[test]
fn un_niveau_inconnu_vaut_info() {
    assert_eq!(niveau_de("verbeux"), tracing::Level::INFO);
    assert_eq!(niveau_de(""), tracing::Level::INFO);
    assert_eq!(niveau_de("ERROR"), tracing::Level::INFO);
}
