use super::{derniere_stable, series_for};

#[test]
fn serie_deduite_de_la_version_du_jeu() {
    assert_eq!(series_for("1.21.1").as_deref(), Some("21.1."));
    assert_eq!(series_for("1.21").as_deref(), Some("21.0."));
    assert_eq!(series_for("1.20.4").as_deref(), Some("20.4."));
    // NeoForge ne couvre pas les versions antérieures au versionnage 1.x.
    assert_eq!(series_for("21w07a"), None);
}

/// Trois règles décident de la version installée, et chacune compte.
///
/// La série : une version pour une autre Minecraft ne démarrera pas. Les
/// bêtas, écartées : elles paraissent dans la même liste, et en installer une
/// par inadvertance change le jeu sous les pieds des joueurs. Le tri par
/// correctif : les versions sont publiées dans l'ordre, mais une
/// republication peut désordonner la liste.
#[test]
fn la_derniere_stable_de_la_serie_est_retenue() {
    let publiees = vec![
        "21.1.9".to_string(),
        "21.1.250".to_string(),
        "21.1.100".to_string(),
        // Une bêta de la même série : jamais installée d'office.
        "21.1.300-beta".to_string(),
        // Une autre série : pour une autre version du jeu.
        "21.4.10".to_string(),
    ];

    assert_eq!(
        derniere_stable(publiees.clone(), "21.1."),
        Some("21.1.250".to_string()),
        "le plus grand correctif de la série, hors bêta"
    );
    assert_eq!(
        derniere_stable(publiees, "21.4."),
        Some("21.4.10".to_string())
    );
}

/// Une série que personne n'a publiée ne donne rien, et ce n'est pas une
/// panne : c'est ce qui arrive le jour d'une sortie de Minecraft, avant que
/// NeoForge ne suive.
#[test]
fn une_serie_sans_version_publiee_ne_donne_rien() {
    assert_eq!(derniere_stable(Vec::new(), "21.1."), None);
    assert_eq!(
        derniere_stable(vec!["21.1.0-beta".to_string()], "21.1."),
        None,
        "une série qui n'a que des bêtas n'a rien de stable"
    );
}
