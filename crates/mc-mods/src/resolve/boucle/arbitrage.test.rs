use super::{Arbitrage, Side, arbitrer, confronter, message_de_remplacement};

/// Le cas qui fait converger la résolution : une dépendance déclarée ne
/// déloge pas ce que le manifeste a épinglé.
#[test]
fn une_demande_moins_autoritaire_ne_deloge_personne() {
    assert_eq!(
        arbitrer(1, 3, false, Side::Both, Side::Client),
        Arbitrage::Conserver {
            cote: Side::Both,
            reprendre: false
        }
    );
}

/// Même build des deux côtés : rien à retélécharger, rien à réécrire. Le
/// demandeur plus autoritaire reprend la place, pour que le verrou nomme celui
/// qui répond réellement de la présence du mod.
#[test]
fn le_meme_build_ne_se_dispute_pas_mais_change_de_demandeur() {
    assert_eq!(
        arbitrer(3, 1, true, Side::Server, Side::Server),
        Arbitrage::Conserver {
            cote: Side::Server,
            reprendre: true
        }
    );
}

#[test]
fn un_demandeur_plus_autoritaire_impose_son_build() {
    assert_eq!(
        arbitrer(3, 1, false, Side::Server, Side::Client),
        Arbitrage::Remplacer { cote: Side::Both }
    );
}

/// Réclamé côté serveur par une branche, côté client par une autre : il doit
/// finir des deux côtés, quelle que soit celle qui l'emporte sur la version.
#[test]
fn le_cote_couvre_toujours_les_deux_usages() {
    for meme_build in [true, false] {
        let cote = match arbitrer(2, 2, meme_build, Side::Client, Side::Server) {
            Arbitrage::Conserver { cote, .. } | Arbitrage::Remplacer { cote } => cote,
        };
        assert_eq!(cote, Side::Both, "même build : {meme_build}");
    }
}

/// À autorité égale, le premier arrivé reste celui qui répond du mod. Céder la
/// place à égalité ferait dépendre le nom inscrit dans le verrou de l'ordre où
/// les branches ont été explorées — deux résolutions du même manifeste
/// donneraient deux verrous différents.
#[test]
fn a_autorite_egale_le_demandeur_en_place_le_reste() {
    assert_eq!(
        arbitrer(2, 2, false, Side::Client, Side::Client),
        Arbitrage::Conserver {
            cote: Side::Client,
            reprendre: false
        }
    );
    // Un cran au-dessus, en revanche, la reprise est due.
    assert_eq!(
        arbitrer(3, 2, true, Side::Client, Side::Client),
        Arbitrage::Conserver {
            cote: Side::Client,
            reprendre: true
        }
    );
}

/// L'avertissement doit nommer les deux versions et qui a tranché : sans cela,
/// le joueur lit « une version a été remplacée », ce qui ne se distingue pas du
/// silence — et le pack n'a pas la version que le manifeste promet.
#[test]
fn l_avertissement_nomme_les_deux_versions_et_la_raison() {
    use crate::resolve::essais::installed;
    use crate::resolve::raison::Reason;

    let ecarte = installed("jei", &["jei"], &[]);
    let mut retenu = ecarte.candidate.clone();
    retenu.version_number = "19.56".into();

    let message = message_de_remplacement(&retenu, &ecarte, &Reason::Explicit);

    assert!(message.contains("jei"), "{message}");
    assert!(
        message.contains("19.56"),
        "le build retenu n'est pas nommé : {message}"
    );
    assert!(
        message.contains("1.0"),
        "le build écarté n'est pas nommé : {message}"
    );
    assert!(
        message.contains("manifeste"),
        "la raison manque : {message}"
    );
}

/// `confronter` applique l'arbitrage à l'entrée en place. Rendre toujours
/// « rien à faire » laisserait le build le moins autoritaire s'installer — et
/// l'épinglage du manifeste ne servirait plus à rien.
#[test]
fn confronter_rend_le_cote_a_inscrire_quand_le_build_est_remplace() {
    use crate::resolve::essais::installed;
    use crate::resolve::raison::Reason;

    let mut en_place = installed("jei", &["jei"], &[]);
    en_place.autorite = 1;
    en_place.side = Side::Client;
    let mut entrant = en_place.candidate.clone();
    entrant.version_id = "v2".into();
    entrant.version_number = "19.56".into();

    let issue = confronter(
        &mut en_place,
        &entrant,
        &Reason::Explicit,
        4,
        Side::Server,
        false,
    );

    assert_eq!(
        issue,
        Some(Side::Both),
        "le côté fusionné doit être inscrit"
    );

    // Et à l'inverse, un demandeur moins autoritaire ne déloge personne.
    let mut en_place = installed("jei", &["jei"], &[]);
    en_place.autorite = 4;
    assert_eq!(
        confronter(
            &mut en_place,
            &entrant,
            &Reason::Explicit,
            1,
            Side::Both,
            false
        ),
        None
    );
}
