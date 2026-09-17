use super::{Features, allowed, allowed_with};
use crate::vanilla::descripteur::{OsCondition, Rule};

fn rule(action: &str, os: Option<&str>, arch: Option<&str>) -> Rule {
    Rule {
        action: action.to_string(),
        os: os.map(|name| OsCondition {
            name: Some(name.to_string()),
            arch: arch.map(str::to_string),
        }),
        features: None,
    }
}

/// Règle conditionnée à un drapeau, comme celles des arguments.
fn feature_rule(action: &str, feature: &str, expected: bool) -> Rule {
    Rule {
        action: action.to_string(),
        os: None,
        features: Some(std::collections::BTreeMap::from([(
            feature.to_string(),
            expected,
        )])),
    }
}

#[test]
fn sans_regle_la_bibliotheque_est_retenue() {
    assert!(allowed(&[], "linux", "x86_64"));
}

#[test]
fn un_argument_sous_drapeau_n_apparait_que_si_le_drapeau_est_actif() {
    // C'est ainsi que --quickPlayMultiplayer reste absent tant qu'on ne
    // demande pas à rejoindre un serveur.
    let rules = vec![feature_rule("allow", "is_quick_play_multiplayer", true)];
    let actifs = Features::from(["is_quick_play_multiplayer".to_string()]);

    assert!(allowed_with(&rules, "linux", "x86_64", &actifs));
    assert!(!allowed_with(&rules, "linux", "x86_64", &Features::new()));
}

#[test]
fn un_drapeau_attendu_faux_exige_son_absence() {
    // « sauf en démo » : la règle s'applique quand le drapeau est absent.
    let rules = vec![feature_rule("allow", "is_demo_user", false)];
    assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));

    let demo = Features::from(["is_demo_user".to_string()]);
    assert!(!allowed_with(&rules, "linux", "x86_64", &demo));
}

#[test]
fn les_regles_de_bibliotheques_ignorent_les_drapeaux() {
    // Elles n'en portent pas : le comportement doit rester celui d'avant.
    let rules = vec![rule("allow", Some("linux"), None)];
    assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));
    assert!(!allowed_with(&rules, "osx", "x86_64", &Features::new()));
}

#[test]
fn une_bibliotheque_reservee_a_macos_est_ecartee_ailleurs() {
    // java-objc-bridge n'a de sens que sur macOS ; l'installer sous Linux
    // alourdit le classpath sans jamais servir.
    let rules = vec![rule("allow", Some("osx"), None)];
    assert!(allowed(&rules, "osx", "x86_64"));
    assert!(!allowed(&rules, "linux", "x86_64"));
}

#[test]
fn la_derniere_regle_applicable_l_emporte() {
    let rules = vec![
        rule("allow", None, None),
        rule("disallow", Some("osx"), None),
    ];
    assert!(allowed(&rules, "linux", "x86_64"));
    assert!(!allowed(&rules, "osx", "arm64"));
}

#[test]
fn une_regle_peut_viser_une_architecture() {
    let rules = vec![rule("allow", Some("windows"), Some("x86"))];
    assert!(allowed(&rules, "windows", "x86"));
    assert!(!allowed(&rules, "windows", "x86_64"));
}
