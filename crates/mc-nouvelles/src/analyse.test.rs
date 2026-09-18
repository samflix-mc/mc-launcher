use super::analyser;
use crate::arbre::{Bloc, Inline};

const FIL: &str = "https://mc-launcher.ggy.info/pack/nouvelles.json";

fn texte(t: &str) -> Inline {
    Inline::Texte { texte: t.into() }
}

fn analyse(markdown: &str) -> Vec<Bloc> {
    analyser(markdown, FIL)
}

// --- Les blocs -------------------------------------------------------------

#[test]
fn un_paragraphe_est_un_paragraphe() {
    assert_eq!(
        analyse("Bonjour."),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("Bonjour.")]
        }]
    );
}

/// Deux lignes de suite appartiennent au MÊME paragraphe : c'est la règle du
/// markdown, et sans elle un billet rédigé avec des retours à la ligne de
/// confort s'afficherait en une suite de paragraphes d'une ligne.
#[test]
fn deux_lignes_de_suite_font_un_seul_paragraphe() {
    assert_eq!(
        analyse("Une ligne\net la suite"),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("Une ligne et la suite")]
        }]
    );
}

#[test]
fn une_ligne_vide_separe_deux_paragraphes() {
    assert_eq!(analyse("Un.\n\nDeux.").len(), 2);
}

/// Le niveau est borné à 2..=4 : le titre du billet occupe le niveau 1 de la
/// page. Un `#` du corps ne doit pas le concurrencer — ni au rendu, ni pour un
/// lecteur d'écran, qui se sert de la hiérarchie pour naviguer.
#[test]
fn les_titres_sont_bornes_a_deux_quatre() {
    for (markdown, attendu) in [
        ("# Un", 2u8),
        ("## Deux", 3),
        ("### Trois", 4),
        ("#### Quatre", 4),
        ("###### Six", 4),
    ] {
        match &analyse(markdown)[0] {
            Bloc::Titre { niveau, .. } => assert_eq!(*niveau, attendu, "{markdown}"),
            autre => panic!("{markdown} n'est pas un titre : {autre:?}"),
        }
    }
}

/// Au-delà de six dièses, ce n'est plus un titre — c'est du texte.
///
/// La borne haute n'est pas décorative : CommonMark s'arrête à six, et une
/// ligne de sept dièses est presque toujours un séparateur décoratif écrit à
/// la main. La rendre en titre de niveau 4 poserait dans la hiérarchie du
/// document — celle que suit un lecteur d'écran — un niveau que personne n'a
/// voulu y mettre.
///
/// Le test porte sur SEPT et non sur six : c'est le premier cas où la borne
/// décide, et le seul qui distingue `diese == 0 || diese > 6` de la même
/// ligne écrite avec un `&&`, qui ne serait jamais vraie et laisserait tout
/// passer.
#[test]
fn au_dela_de_six_dieses_ce_n_est_plus_un_titre() {
    for markdown in ["####### Sept", "######## Huit"] {
        assert_eq!(
            analyse(markdown),
            vec![Bloc::Paragraphe {
                contenu: vec![texte(markdown)]
            }],
            "{markdown}"
        );
    }
}

/// Un dièse SANS espace n'est pas un titre : c'est un mot-dièse, et les
/// billets en contiennent.
#[test]
fn un_diese_sans_espace_n_est_pas_un_titre() {
    assert_eq!(
        analyse("#saison3 arrive"),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("#saison3 arrive")]
        }]
    );
}

#[test]
fn les_puces_font_une_seule_liste() {
    assert_eq!(
        analyse("- un\n- deux\n- trois"),
        vec![Bloc::Liste {
            puces: vec![vec![texte("un")], vec![texte("deux")], vec![texte("trois")]]
        }]
    );
    // L'astérisque aussi : les deux formes existent dans la nature.
    assert_eq!(analyse("* un\n* deux").len(), 1);
}

/// Une ligne ordinaire après une liste la FERME. Sans cela, tout le reste du
/// billet deviendrait une puce.
#[test]
fn une_ligne_ordinaire_ferme_la_liste() {
    let blocs = analyse("- un\n- deux\nUne phrase.");
    assert_eq!(blocs.len(), 2);
    assert!(matches!(blocs[0], Bloc::Liste { .. }));
    assert!(matches!(blocs[1], Bloc::Paragraphe { .. }));
}

#[test]
fn trois_tirets_font_un_separateur() {
    assert_eq!(analyse("---"), vec![Bloc::Separateur]);
    assert_eq!(analyse("-----"), vec![Bloc::Separateur]);
    // Deux tirets n'en font pas un : c'est du texte.
    assert!(matches!(analyse("--")[0], Bloc::Paragraphe { .. }));
}

#[test]
fn un_corps_vide_ne_donne_aucun_bloc() {
    assert!(analyse("").is_empty());
    assert!(analyse("\n\n  \n").is_empty());
}

// --- Les inlines -----------------------------------------------------------

#[test]
fn le_gras_l_italique_et_le_code() {
    assert_eq!(
        analyse("a **gras** b *ital* c `code` d"),
        vec![Bloc::Paragraphe {
            contenu: vec![
                texte("a "),
                Inline::Gras {
                    texte: "gras".into()
                },
                texte(" b "),
                Inline::Italique {
                    texte: "ital".into()
                },
                texte(" c "),
                Inline::Code {
                    texte: "code".into()
                },
                texte(" d"),
            ]
        }]
    );
}

/// L'ORDRE des délimiteurs n'est pas décoratif : `**gras**` commence par `*`,
/// et tester l'italique d'abord produirait un italique vide, puis le mot, puis
/// un italique vide.
#[test]
fn le_gras_l_emporte_sur_l_italique() {
    match &analyse("**important**")[0] {
        Bloc::Paragraphe { contenu } => {
            assert_eq!(contenu.len(), 1, "{contenu:?}");
            assert!(matches!(contenu[0], Inline::Gras { .. }), "{contenu:?}");
        }
        autre => panic!("{autre:?}"),
    }
}

/// Un délimiteur jamais refermé est un caractère ordinaire. Un `*` isolé au
/// milieu d'une phrase — « 3 * 4 » — ne doit pas mettre tout le reste du
/// billet en italique.
#[test]
fn un_delimiteur_jamais_referme_est_du_texte() {
    assert_eq!(
        analyse("3 * 4 = 12"),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("3 * 4 = 12")]
        }]
    );
}

/// `****` n'est pas une mise en forme vide : c'est du texte. Produire un nœud
/// vide obligerait le front à savoir ne pas le rendre.
#[test]
fn un_delimiteur_vide_est_du_texte() {
    assert_eq!(
        analyse("****"),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("****")]
        }]
    );
}

// --- Ce que le module existe pour empêcher ---------------------------------

/// LA garantie du crate : ce qui n'est pas reconnu devient du TEXTE, jamais du
/// balisage. Un `<script>` écrit dans un billet ressort comme les caractères
/// qu'il est, et le front l'affichera comme tel.
///
/// C'est la moitié Rust de la condition à laquelle le CSP a été desserré.
#[test]
fn aucun_balisage_ne_traverse_l_analyse() {
    let venimeux = "<script>alert(1)</script> et <img onerror=alert(2)>";
    match &analyse(venimeux)[0] {
        Bloc::Paragraphe { contenu } => {
            assert_eq!(contenu, &vec![texte(venimeux)]);
        }
        autre => panic!("{autre:?}"),
    }
}

/// Un lien acceptable devient un `Lien` avec son href.
#[test]
fn un_lien_devient_un_lien() {
    match &analyse("voir [le mod](https://modrinth.com/mod/jei) ici")[0] {
        Bloc::Paragraphe { contenu } => {
            assert_eq!(contenu[0], texte("voir "));
            assert_eq!(
                contenu[1],
                Inline::Lien {
                    texte: "le mod".into(),
                    href: "https://modrinth.com/mod/jei".into()
                }
            );
            assert_eq!(contenu[2], texte(" ici"));
        }
        autre => panic!("{autre:?}"),
    }
}

/// Un lien refusé ne fait pas disparaître la PHRASE : le libellé devient du
/// texte ordinaire. Effacer une phrase parce que son lien déplaît serait pire
/// que de la garder sans lien.
#[test]
fn un_lien_refuse_garde_son_libelle_en_texte() {
    match &analyse("cliquez [ici](javascript:alert(1)) vite")[0] {
        Bloc::Paragraphe { contenu } => {
            assert!(
                !contenu.iter().any(|i| matches!(i, Inline::Lien { .. })),
                "un lien dangereux a survécu : {contenu:?}"
            );
            let tout: String = contenu
                .iter()
                .map(|i| match i {
                    Inline::Texte { texte } => texte.clone(),
                    autre => format!("{autre:?}"),
                })
                .collect();
            assert!(tout.contains("ici"), "le libellé a disparu : {tout}");
            assert!(tout.contains("cliquez"), "{tout}");
        }
        autre => panic!("{autre:?}"),
    }
}

/// Un crochet qui n'ouvre pas un lien reste un crochet.
#[test]
fn un_crochet_seul_est_du_texte() {
    assert_eq!(
        analyse("un [truc] sans lien"),
        vec![Bloc::Paragraphe {
            contenu: vec![texte("un [truc] sans lien")]
        }]
    );
}
