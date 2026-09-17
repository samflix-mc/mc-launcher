use super::{Arbitrage, Side, arbitrer};

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
