use super::annoncer;
use crate::commandes::essais::{Atelier, entree, verrou};

/// Le verrou est le produit de la commande : ce qu'on affiche est ce qu'une
/// revue lira. L'affichage part sur la sortie standard ; ce que le test retient
/// est qu'il traverse ses trois cas — sans précédent, inchangé, avec
/// changements — sans paniquer.
#[test]
fn le_rapport_traverse_ses_trois_cas() {
    let atelier = Atelier::neuf("rapport");
    let chemin = atelier.racine.join("samflix.lock.json");

    let avant = verrou(vec![entree("jei", "both")]);
    let apres = verrou(vec![entree("jei", "both"), entree("sodium", "client")]);

    // Premier verrou : rien à comparer.
    annoncer(&apres, &chemin, None);
    // Inchangé : la section « Changements » reste absente.
    annoncer(&apres, &chemin, Some(&apres));
    // Un mod de plus : il doit apparaître.
    annoncer(&apres, &chemin, Some(&avant));
}

/// Une dépendance introuvable est signalée là où on la verra, y compris depuis
/// le rapport de `lock`.
#[test]
fn le_rapport_signale_les_dependances_introuvables() {
    let atelier = Atelier::neuf("rapport-manques");
    let chemin = atelier.racine.join("samflix.lock.json");

    let mut lock = verrou(vec![entree("jei", "both")]);
    lock.unresolved.push(mc_pack::lockfile::LockedMissing {
        mod_id: "bookshelf".into(),
        required_by: "jei".into(),
        side: "both".into(),
    });

    annoncer(&lock, &chemin, None);
}

/// Un verrou vide n'est pas une faute : c'est un pack sans mod, et la commande
/// doit s'en accommoder.
#[test]
fn un_verrou_sans_mod_s_annonce_quand_meme() {
    let atelier = Atelier::neuf("rapport-vide");
    annoncer(
        &verrou(Vec::new()),
        &atelier.racine.join("samflix.lock.json"),
        None,
    );
}
