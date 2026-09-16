use super::*;
use crate::{Channel, Origin, Side};

fn candidat(sha1: Option<&str>, sha512: Option<&str>) -> Candidate {
    Candidate {
        origin: Origin::Modrinth,
        project_id: "p".into(),
        slug: "s".into(),
        name: "N".into(),
        version_id: "v".into(),
        version_number: "1.0".into(),
        display_name: "1.0".into(),
        channel: Channel::Release,
        file_name: "s.jar".into(),
        url: "https://exemple.invalid/s.jar".into(),
        sha1: sha1.map(str::to_string),
        sha512: sha512.map(str::to_string),
        size: 0,
        published: "2025-01-01".into(),
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: None,
        redistributable: true,
    }
}

/// Ce qui vaut pour la majeure partie d'un pack : Modrinth donne les deux,
/// et c'est la forte qu'on oppose au fichier téléchargé.
#[test]
fn l_empreinte_forte_prime_sur_le_sha1() {
    let c = candidat(Some("aa"), Some("bb"));
    assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha512("bb".into())));
}

/// CurseForge ne publie rien de plus fort : refuser le SHA-1 reviendrait à
/// installer ses jars sans les vérifier du tout.
#[test]
fn a_defaut_le_sha1_reste_opposable() {
    let c = candidat(Some("aa"), None);
    assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha1("aa".into())));
    assert_eq!(candidat(None, None).checksum(), None);
}
