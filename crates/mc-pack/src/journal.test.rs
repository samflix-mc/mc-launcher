use super::{conclure, ouvrir};
use mc_pack::source::Source;

/// Le span racine porte le nom de la commande et la provenance du pack : dans
/// le fichier comme dans Sentry, une exécution se lit d'un bloc même quand
/// plusieurs se succèdent.
#[test]
fn le_span_racine_nomme_la_commande_et_le_pack() {
    let layout = mc_instance::Layout::new(std::env::temp_dir().join("mc-pack-journal"));
    let source = Source::parse("packs/samflix.json", &layout);

    // Le span est entré puis quitté ; ce que le test retient est qu'il
    // s'assemble et se referme, y compris sans souscripteur posé.
    let entree = ouvrir("install", &source);
    tracing::info!("une étape de la commande");
    drop(entree);
}

/// Une erreur remontée jusqu'ici met fin au programme : c'est le dernier
/// endroit où elle peut devenir un incident plutôt qu'un simple message.
#[test]
fn les_deux_issues_d_une_commande_sont_journalisees() {
    let debut = std::time::Instant::now();
    // Un guard sans journal : `log_path` rend alors `None`, et la conclusion
    // ne doit pas pour autant renvoyer vers un fichier inexistant.
    let log = mc_log::Guard::sans_journal();

    conclure("verify", &Ok(std::process::ExitCode::SUCCESS), debut, &log);
    conclure(
        "install",
        &Err(anyhow::anyhow!("le pack est injoignable")),
        debut,
        &log,
    );
}
