use super::*;
use crate::Origin;

fn cle(source: Origin, projet: &str) -> Cle {
    (source, projet.to_string())
}


#[test]
fn le_manifeste_passe_avant_les_dependances_meme_poussees_en_cours_de_route() {
    // Le défaut d'origine tenait entièrement ici. En pile unique, la
    // dépendance poussée par « b » passait avant la demande « a » restante :
    // « a » était résolu sans son épinglage, et la demande épinglée trouvait
    // ensuite la clé prise. Le joueur installait un autre build que celui du
    // verrou, sans que rien ne le signale.
    let mut queue = FileDeResolution::default();
    queue.pousser(Request::new("a"), Reason::Explicit, None);
    queue.pousser(Request::new("b"), Reason::Explicit, None);

    let premier = queue.suivante().unwrap();
    assert_eq!(premier.request.slug, "b");
    queue.pousser(
        Request::new("a"),
        Reason::Declared { by: "b".into() },
        Some(cle(Origin::Modrinth, "b")),
    );

    let ensuite = queue.suivante().unwrap();
    assert_eq!(ensuite.request.slug, "a");
    assert_eq!(ensuite.reason, Reason::Explicit);

    let enfin = queue.suivante().unwrap();
    assert_eq!(enfin.request.slug, "a");
    assert_eq!(enfin.reason, Reason::Declared { by: "b".into() });
    assert!(queue.est_vide());
}
