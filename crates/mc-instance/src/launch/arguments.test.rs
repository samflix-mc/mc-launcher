use super::substitute;

use super::*;

fn vars() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("auth_player_name".into(), "Sam".into()),
        ("game_directory".into(), "/jeu".into()),
        ("classpath_separator".into(), ":".into()),
    ])
}

#[test]
fn substitution_simple() {
    assert_eq!(substitute("${auth_player_name}", &vars()), "Sam");
    assert_eq!(
        substitute("--gameDir=${game_directory}", &vars()),
        "--gameDir=/jeu"
    );
}

#[test]
fn plusieurs_variables_dans_un_argument() {
    // Le module path de NeoForge en enchaîne une dizaine.
    let rendu = substitute("a${classpath_separator}b${classpath_separator}c", &vars());
    assert_eq!(rendu, "a:b:c");
}

#[test]
fn une_variable_inconnue_reste_visible() {
    // La vider décalerait les arguments suivants sans rien signaler.
    assert_eq!(substitute("${inconnue}", &vars()), "${inconnue}");
}

#[test]
fn un_texte_sans_variable_est_intact() {
    assert_eq!(substitute("--add-modules", &vars()), "--add-modules");
}

#[test]
fn une_accolade_non_fermee_ne_fait_pas_paniquer() {
    assert_eq!(substitute("${tronque", &vars()), "${tronque");
}
