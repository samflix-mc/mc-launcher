use super::*;
use crate::resolve::essais::candidat;

/// Le cas de CurseForge, qui ne publie pas de SHA-512 : sans cette reprise, un
/// jar déjà vérifié une fois repartirait sans empreinte au passage suivant.
#[test]
fn le_verrou_comble_ce_que_la_source_ne_publie_pas() {
    let mut c = candidat("jade", "15.10.6");
    let mut request = Request::new("jade");
    request.expected_sha1 = Some("aa".into());
    request.expected_sha512 = Some("bb".into());

    completer_empreintes(&mut c, &request);

    assert_eq!(c.sha1.as_deref(), Some("aa"));
    assert_eq!(c.sha512.as_deref(), Some("bb"));
}

/// Ce que la source publie fait foi : une empreinte figée dans un verrou plus
/// ancien ne doit pas écraser celle du build qu'on vient de résoudre.
#[test]
fn ce_que_la_source_publie_n_est_pas_ecrase() {
    let mut c = candidat("jade", "15.10.6");
    c.sha512 = Some("celle-de-la-source".into());
    let mut request = Request::new("jade");
    request.expected_sha512 = Some("celle-du-verrou".into());

    completer_empreintes(&mut c, &request);

    assert_eq!(c.sha512.as_deref(), Some("celle-de-la-source"));
}

/// Un identifiant Modrinth n'existe pas chez CurseForge : une dépendance se
/// cherche dans la source de son parent, jamais ailleurs.
#[test]
fn une_dependance_se_cherche_dans_la_source_de_son_parent() {
    let dep = DeclaredDep {
        project_id: "bookshelf".into(),
        version_id: Some("abc".into()),
    };
    let request = dependance(dep, Origin::CurseForge);

    assert_eq!(request.slug, "bookshelf");
    assert_eq!(request.source, Some(Origin::CurseForge));
    assert_eq!(request.file.as_deref(), Some("abc"));
    // Une dépendance accepte la beta : beaucoup de bibliothèques ne publient
    // que dans ce canal, et les refuser bloquerait le pack entier.
    assert_eq!(request.channel, Some(Channel::Beta));
}
