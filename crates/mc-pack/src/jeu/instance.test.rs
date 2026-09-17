use super::ouvrir;
use crate::essais::{Atelier, MANIFESTE, entree};

/// Ouvre ce qui est installé, pas ce qui est publié : aller chercher le pack du
/// jour décrirait des mods que le dossier ne contient pas, et interdirait de
/// jouer sans réseau.
#[test]
fn l_instance_ouverte_est_celle_qui_est_installee() {
    let atelier = Atelier::neuf("ouvrir-ok");
    let source = atelier.pack_installe(vec![entree("jei", "both", None)]);

    let (manifeste, lock, instance) = ouvrir(&source, &atelier.options()).expect("tout est là");

    assert_eq!(manifeste.name, "samflix");
    assert_eq!(lock.mods.len(), 1);
    assert_eq!(instance.name, "samflix");
    assert!(instance.mods_dir().join("jei.jar").is_file());
}

/// Sans verrou, il n'y a pas de partie à lancer, et le message doit dire quoi
/// faire.
#[test]
fn sans_verrou_on_renvoie_a_l_installation() {
    let atelier = Atelier::neuf("ouvrir-sans-verrou");
    let manifeste = atelier.racine.join("samflix.json");
    std::fs::write(&manifeste, MANIFESTE).unwrap();
    let options = atelier.options();
    let source = crate::source::Source::parse(manifeste.to_str().unwrap(), &options.layout);

    let erreur = ouvrir(&source, &options).expect_err("aucun verrou");
    assert!(
        format!("{erreur:#}").contains("mc-pack install"),
        "{erreur:#}"
    );
}

/// À défaut de `--instance`, c'est le nom du pack qui sert.
#[test]
fn a_defaut_de_nom_l_instance_porte_celui_du_pack() {
    let atelier = Atelier::neuf("ouvrir-nom");
    let source = atelier.pack_installe(vec![entree("jei", "both", None)]);
    let mut options = atelier.options();
    options.instance_name = None;

    let (_, _, instance) = ouvrir(&source, &options).expect("tout est là");
    assert_eq!(instance.name, "samflix");
}

/// Une instance qui ne correspond pas au verrou est refusée avant de lancer :
/// démarrer quand même, c'est laisser le serveur trancher par une éjection qui
/// ne nomme pas sa cause.
#[test]
fn une_instance_divergente_est_refusee_avant_le_lancement() {
    let atelier = Atelier::neuf("ouvrir-divergent");
    let source = atelier.pack_installe(vec![entree("jei", "both", None)]);
    let instance = atelier.options().layout.instance("samflix");
    std::fs::remove_file(instance.mods_dir().join("jei.jar")).unwrap();

    let erreur = ouvrir(&source, &atelier.options()).expect_err("l'instance diverge");
    assert!(format!("{erreur:#}").contains("jei.jar"), "{erreur:#}");
}
