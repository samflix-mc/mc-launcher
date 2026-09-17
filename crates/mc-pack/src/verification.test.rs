use super::verify;
use crate::Options;
use crate::essais::{Atelier, MANIFESTE, entree, manque, verrou};
use crate::lockfile::{LockedMod, Lockfile};
use crate::source::Source;

const JAR: &[u8] = b"le jar";

/// Une installation complète : le pack, son verrou, et les fichiers Mojang que
/// `mc_instance::verify` exige.
fn installation(nom: &str, mods: Vec<LockedMod>) -> (Atelier, Options, Source) {
    let atelier = Atelier::neuf(nom);
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());
    verrou(mods)
        .save(&atelier.racine.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(atelier.racine.join("données")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };

    // Ce que mc-instance vérifie de son côté : descripteurs et client.
    let shared = options.layout.shared();
    for (chemin, contenu) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let cible = shared.join(chemin);
        std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
        std::fs::write(cible, contenu).unwrap();
    }

    let source = Source::parse(manifeste.to_str().unwrap(), &options.layout);
    (atelier, options, source)
}

/// Pose un jar dans le dossier `mods` du côté demandé.
fn poser(options: &Options, cote: &str, nom: &str, contenu: &[u8]) {
    let instance = options.layout.instance("samflix");
    let dossier = match cote {
        "client" => instance.mods_dir(),
        _ => instance.dir.join("server").join("mods"),
    };
    std::fs::create_dir_all(&dossier).unwrap();
    std::fs::write(dossier.join(nom), contenu).unwrap();
}

#[test]
fn une_installation_complete_ne_signale_rien() {
    let (_atelier, options, source) =
        installation("verif-ok", vec![entree("jei", "both", Some(JAR))]);
    poser(&options, "client", "jei.jar", JAR);
    poser(&options, "server", "jei.jar", JAR);

    let problemes = verify(&source, &options, false).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}

/// Un mod client n'a rien à faire dans le dossier du serveur, et
/// réciproquement : le vérifier des deux côtés ferait crier sur une
/// installation correcte.
#[test]
fn un_mod_client_n_est_cherche_que_du_cote_client() {
    let (_atelier, options, source) =
        installation("verif-cote", vec![entree("sodium", "client", Some(JAR))]);
    poser(&options, "client", "sodium.jar", JAR);

    let problemes = verify(&source, &options, false).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}

#[test]
fn un_mod_manquant_est_nomme_par_son_chemin() {
    let (_atelier, options, source) =
        installation("verif-manquant", vec![entree("jei", "both", Some(JAR))]);
    poser(&options, "client", "jei.jar", JAR);
    // Rien côté serveur.

    let problemes = verify(&source, &options, false).unwrap();
    assert_eq!(problemes.len(), 1, "{problemes:?}");
    assert!(problemes[0].contains("mod manquant"), "{problemes:?}");
    assert!(problemes[0].contains("jei.jar"), "{problemes:?}");
}

/// Un jar altéré doit être vu : c'est tout l'intérêt de garder l'empreinte
/// dans le verrou.
#[test]
fn un_mod_altere_est_signale_avec_les_deux_empreintes() {
    let (_atelier, options, source) =
        installation("verif-altere", vec![entree("jei", "client", Some(JAR))]);
    poser(&options, "client", "jei.jar", b"autre chose");

    let problemes = verify(&source, &options, false).unwrap();
    assert_eq!(problemes.len(), 1, "{problemes:?}");
    assert!(problemes[0].contains("empreinte"), "{problemes:?}");
}

/// Un verrou sans empreinte ne permet pas de vérifier : c'est le cas des
/// entrées écrites depuis une source qui n'en publiait pas.
#[test]
fn un_mod_sans_empreinte_est_seulement_constate_present() {
    let (_atelier, options, source) =
        installation("verif-sans-empreinte", vec![entree("jei", "client", None)]);
    poser(&options, "client", "jei.jar", b"peu importe");

    let problemes = verify(&source, &options, false).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}

/// Le verrou porte les `modId` fournis par chaque jar : la cohérence de
/// l'ensemble se vérifie sans rouvrir une seule archive.
#[test]
fn une_dependance_non_satisfaite_est_signalee_sans_ouvrir_de_jar() {
    let atelier = Atelier::neuf("verif-coherence");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());
    let mut lock = verrou(vec![entree("jei", "client", Some(JAR))]);
    lock.unresolved.push(manque("bookshelf", "jei"));
    lock.save(&atelier.racine.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(atelier.racine.join("données")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };
    let shared = options.layout.shared();
    for (chemin, contenu) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let cible = shared.join(chemin);
        std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
        std::fs::write(cible, contenu).unwrap();
    }
    poser(&options, "client", "jei.jar", JAR);

    let source = Source::parse(manifeste.to_str().unwrap(), &options.layout);
    let problemes = verify(&source, &options, false).unwrap();

    assert!(
        problemes
            .iter()
            .any(|p| p.contains("dépendance non satisfaite") && p.contains("bookshelf")),
        "{problemes:?}"
    );
}

/// Un manque que le verrou consigne mais qu'un autre jar fournit finalement
/// n'en est pas un : le signaler ferait chercher un problème réglé.
#[test]
fn un_manque_finalement_fourni_n_est_plus_un_probleme() {
    let atelier = Atelier::neuf("verif-manque-comble");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());
    let mut lock = verrou(vec![entree("bookshelf", "client", Some(JAR))]);
    lock.unresolved.push(manque("bookshelf", "jei"));
    lock.save(&atelier.racine.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(atelier.racine.join("données")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };
    let shared = options.layout.shared();
    for (chemin, contenu) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let cible = shared.join(chemin);
        std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
        std::fs::write(cible, contenu).unwrap();
    }
    poser(&options, "client", "bookshelf.jar", JAR);

    let source = Source::parse(manifeste.to_str().unwrap(), &options.layout);
    let problemes = verify(&source, &options, false).unwrap();

    assert!(
        !problemes.iter().any(|p| p.contains("non satisfaite")),
        "{problemes:?}"
    );
}

/// Sans verrou, il n'y a rien à vérifier — et le message doit dire quoi faire.
#[test]
fn sans_verrou_la_verification_renvoie_a_l_installation() {
    let atelier = Atelier::neuf("verif-sans-verrou");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());
    let options = Options {
        layout: mc_instance::Layout::new(atelier.racine.join("données")),
        ..Default::default()
    };
    let source = Source::parse(manifeste.to_str().unwrap(), &options.layout);

    let erreur = verify(&source, &options, false).expect_err("aucun verrou");
    assert!(
        format!("{erreur:#}").contains("mc-pack install"),
        "{erreur:#}"
    );
}

/// À défaut de nom d'instance, c'est celui du pack qui sert — sinon la
/// vérification regarderait un répertoire vide et crierait sur tout.
#[test]
fn a_defaut_de_nom_l_instance_porte_celui_du_pack() {
    let (_atelier, mut options, source) =
        installation("verif-nom", vec![entree("jei", "client", Some(JAR))]);
    options.instance_name = None;
    poser(&options, "client", "jei.jar", JAR);

    let problemes = verify(&source, &options, false).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}

/// Le verrou relu doit être celui qu'on a écrit : un pack dont le verrou a été
/// remplacé par autre chose ne doit pas passer pour vérifié.
#[test]
fn le_verrou_lu_est_bien_celui_du_pack() {
    let (atelier, _options, _source) =
        installation("verif-identite", vec![entree("jei", "client", Some(JAR))]);
    let relu = Lockfile::load(&atelier.racine.join("samflix.lock.json")).unwrap();
    assert_eq!(relu.name, "samflix");
}
