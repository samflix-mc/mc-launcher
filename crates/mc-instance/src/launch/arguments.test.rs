use super::BTreeMap;
use super::substitute;

fn vars() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("auth_player_name".into(), "Sam".into()),
        ("game_directory".into(), "/game".into()),
        ("classpath_separator".into(), ":".into()),
    ])
}

#[test]
fn simple_substitution() {
    assert_eq!(substitute("${auth_player_name}", &vars()), "Sam");
    assert_eq!(
        substitute("--gameDir=${game_directory}", &vars()),
        "--gameDir=/game"
    );
}

#[test]
fn several_variables_in_one_argument() {
    // NeoForge's path module chains together about ten of these.
    let rendered = substitute("a${classpath_separator}b${classpath_separator}c", &vars());
    assert_eq!(rendered, "a:b:c");
}

#[test]
fn an_unknown_variable_stays_visible() {
    // Clearing it would shift the following arguments without any signal.
    assert_eq!(substitute("${unknown}", &vars()), "${unknown}");
}

#[test]
fn text_without_a_variable_is_untouched() {
    assert_eq!(substitute("--add-modules", &vars()), "--add-modules");
}

#[test]
fn an_unclosed_brace_does_not_panic() {
    assert_eq!(substitute("${truncated", &vars()), "${truncated");
}
