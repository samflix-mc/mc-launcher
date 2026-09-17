use super::verifier;
use crate::commandes::essais::{Atelier, JAR, entree, verrou};

/// Le verrou vient du cache de ce pack-ci ; l'instance porte le nom du pack et
/// il n'y en a qu'une pour les trois environnements. Un `install` lancé avec un
/// autre `SAMFLIX_ENV` a donc pu remplacer ces jars sans que ce verrou en sache
/// rien.
#[test]
fn une_instance_installee_depuis_un_autre_pack_est_refusee() {
    let atelier = Atelier::neuf("coherence-divergente");
    atelier.pack_installe(vec![entree("jei", "both")]);
    let instance = atelier.options().layout.instance("samflix");

    // Le verrou qu'on lit annonce un mod que l'instance n'a pas.
    let lock = verrou(vec![entree("jei", "both"), entree("sodium", "client")]);

    let erreur = verifier(&lock, &instance).expect_err("l'instance diverge");
    let texte = format!("{erreur:#}");
    assert!(texte.contains("sodium.jar"), "{texte}");
    assert!(texte.contains("mc-pack install"), "{texte}");
    // L'environnement et sa provenance sont dits : c'est ce qui permet de
    // comprendre pourquoi les deux divergent.
    assert!(texte.contains("environnement"), "{texte}");
}

#[test]
fn une_instance_conforme_laisse_passer() {
    let atelier = Atelier::neuf("coherence-ok");
    atelier.pack_installe(vec![entree("jei", "both")]);
    let instance = atelier.options().layout.instance("samflix");

    verifier(&verrou(vec![entree("jei", "both")]), &instance).expect("tout est là");
}

/// Un mod serveur absent du dossier client n'est pas une divergence : il n'a
/// rien à y faire.
#[test]
fn un_mod_serveur_ne_compte_pas_cote_client() {
    let atelier = Atelier::neuf("coherence-serveur");
    atelier.pack_installe(Vec::new());
    let instance = atelier.options().layout.instance("samflix");
    std::fs::write(instance.mods_dir().join("jei.jar"), JAR).unwrap();

    let lock = verrou(vec![entree("jei", "client"), entree("ledger", "server")]);
    verifier(&lock, &instance).expect("seul le côté client compte ici");
}
