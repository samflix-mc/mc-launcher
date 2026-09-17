use super::diagnostic;
use crate::commandes::essais::environnement;
use std::process::ExitCode;

fn reussite(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

/// Première chose à demander à quelqu'un dont l'installation échoue : la
/// réponse tient en dix lignes et dit où trouver le reste. Elle doit donc
/// sortir même quand il n'y a pas de journal — c'est-à-dire précisément quand
/// le répertoire n'est pas inscriptible.
#[test]
fn le_diagnostic_sort_meme_sans_fichier_de_journal() {
    let _env = environnement("local");
    let log = mc_log::Guard::sans_journal();

    assert!(reussite(diagnostic(&log, false).unwrap()));
}

/// Le diagnostic annonce le journal qu'il y a, pas celui qu'il devrait y
/// avoir : citer un fichier absent enverrait chercher pour rien.
#[test]
fn le_diagnostic_annonce_le_journal_quand_il_y_en_a_un() {
    let _env = environnement("production");
    let dir = std::env::temp_dir().join(format!("mc-pack-diag-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let log = mc_log::Guard::new_pour_essais(dir.clone(), "mc-pack");
    assert!(log.log_path().is_some());
    assert!(reussite(diagnostic(&log, false).unwrap()));

    std::fs::remove_dir_all(&dir).ok();
}

/// Avec la télémétrie coupée, `--incident-test` n'a rien à envoyer et le dit,
/// au lieu d'attendre dix secondes une file qui ne partira jamais.
#[test]
fn sans_telemetrie_l_incident_de_test_ne_tente_rien() {
    let _env = environnement("local");
    let log = mc_log::Guard::sans_journal();

    // SAFETY : posée et retirée ici ; le verrou d'environnement sérialise les
    // tests de ce binaire qui touchent à la configuration.
    let precedent = std::env::var_os("SAMFLIX_TELEMETRY");
    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "0");
    }

    let code = diagnostic(&log, true).unwrap();

    unsafe {
        match precedent {
            Some(valeur) => std::env::set_var("SAMFLIX_TELEMETRY", valeur),
            None => std::env::remove_var("SAMFLIX_TELEMETRY"),
        }
    }

    assert!(reussite(code), "rien à envoyer n'est pas un échec");
}
