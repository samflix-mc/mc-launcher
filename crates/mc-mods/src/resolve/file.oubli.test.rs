//! Ce que la file retire quand un build cède la place.

use super::{Cle, FileDeResolution, Reason, Request};
use crate::Origin;

fn cle(source: Origin, projet: &str) -> Cle {
    (source, projet.to_string())
}

#[test]
fn remplacer_un_build_oublie_les_dependances_de_celui_qu_on_ecarte() {
    // Sinon on installe les bibliothèques de la version écartée en plus de
    // celles de la version retenue, et le verrou les consigne comme
    // dépendances d'un build qui n'est pas là.
    let mut queue = FileDeResolution::default();
    let x = cle(Origin::Modrinth, "X");
    let y = cle(Origin::Modrinth, "Y");
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.pousser(
        Request::new("libB"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.pousser(
        Request::new("libC"),
        Reason::Declared { by: "Y".into() },
        Some(y),
    );

    queue.oublier_dependances_de(&x);

    // Celles d'un autre demandeur restent : Y n'a pas été remplacé.
    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libC");
    assert!(queue.est_vide());
}

#[test]
fn oublier_les_dependances_epargne_l_homonyme_venu_de_l_autre_source() {
    // « Jade » se publie sous le même titre chez les deux sources, et les
    // deux projets coexistent jusqu'à la déduplication finale. Retirer les
    // dépendances par le titre emportait celles du jumeau, que plus rien ne
    // repoussait : le pack partait sans sa bibliothèque.
    let mut queue = FileDeResolution::default();
    let modrinth = cle(Origin::Modrinth, "nvQzSEkR");
    let curseforge = cle(Origin::CurseForge, "324717");
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "Jade".into() },
        Some(modrinth.clone()),
    );
    queue.pousser(
        Request::new("libB"),
        Reason::Declared { by: "Jade".into() },
        Some(curseforge),
    );

    queue.oublier_dependances_de(&modrinth);

    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libB");
    assert!(queue.est_vide());
}

#[test]
fn oublier_les_dependances_epargne_le_manifeste() {
    // Un mod du manifeste qui porte le nom d'un parent remplacé n'a pas à
    // disparaître : sa demande ne vient pas de ce parent.
    let mut queue = FileDeResolution::default();
    let x = cle(Origin::Modrinth, "X");
    queue.pousser(Request::new("libA"), Reason::Explicit, None);
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );

    queue.oublier_dependances_de(&x);

    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libA");
    assert_eq!(reste.reason, Reason::Explicit);
    assert!(queue.est_vide());
}
