use super::meme_build;
use crate::resolve::essais::installed;

/// C'est l'identifiant de version qui dit si deux demandes désignent le même
/// build — pas le numéro affiché, que deux sources peuvent partager. De cette
/// réponse dépendent l'avertissement de remplacement et la détection des
/// impasses : l'inverser ferait annoncer des remplacements qui n'en sont pas.
#[test]
fn c_est_l_identifiant_de_version_qui_fait_le_meme_build() {
    let en_place = installed("jei", &["jei"], &[]);

    let identique = en_place.candidate.clone();
    assert!(meme_build(&en_place, &identique));

    let mut autre = en_place.candidate.clone();
    autre.version_id = "v2".into();
    assert!(!meme_build(&en_place, &autre));

    // Même numéro affiché, autre build : c'est le cas de deux sources qui
    // publient la même version sous des identifiants différents.
    let mut homonyme = en_place.candidate.clone();
    homonyme.version_id = "modrinth-xyz".into();
    assert_eq!(homonyme.version_number, en_place.candidate.version_number);
    assert!(!meme_build(&en_place, &homonyme));
}
