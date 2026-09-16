use super::{filtre, layer};

/// Le défaut doit rester bavard à `info` et muet sur les bibliothèques
/// réseau : sans cela, une seule requête HTTP noie le compte rendu.
#[test]
fn sans_rust_log_la_console_parle_a_info() {
    let directives = filtre(None).to_string();
    assert!(directives.contains("info"), "directives : {directives}");
    assert!(directives.contains("hyper"), "directives : {directives}");
}

/// Recopier le « .env » d'exemple tel quel pose une RUST_LOG vide. Traitée
/// comme une directive, elle rendrait la console muette — défaut compris —
/// sans que rien ne l'explique.
#[test]
fn une_rust_log_vide_vaut_une_rust_log_absente() {
    let defaut = filtre(None).to_string();
    assert_eq!(filtre(Some("")).to_string(), defaut);
    assert_eq!(filtre(Some("   ")).to_string(), defaut);
}

#[test]
fn une_rust_log_lisible_est_respectee() {
    let directives = filtre(Some("trace")).to_string();
    assert!(directives.contains("trace"), "directives : {directives}");
}

/// Une RUST_LOG mal écrite retombe sur le défaut plutôt que de tout couper,
/// et le dit sur la sortie d'erreur : le souscripteur n'est pas encore posé,
/// c'est le seul canal disponible.
#[test]
fn une_rust_log_illisible_retombe_sur_le_defaut() {
    assert_eq!(filtre(Some("=====")).to_string(), filtre(None).to_string());
}

#[test]
fn la_couche_console_se_construit() {
    // Elle ne s'inspecte pas — les types de tracing-subscriber sont opaques
    // une fois boxés. Ce que le test retient, c'est qu'assembler le format,
    // l'écrivain censurant et le filtre ne panique pas.
    let _couche = layer();
}
