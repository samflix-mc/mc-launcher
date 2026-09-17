use super::lignes;
use crate::commandes::essais::{Atelier, entree, verrou};

/// Un résultat d'installation, réduit à ce que le compte rendu regarde.
fn resultat(atelier: &Atelier) -> mc_pack::Outcome {
    let options = atelier.options();
    mc_pack::Outcome {
        instance: options.layout.instance("samflix"),
        server_dir: atelier.racine.join("serveur"),
        java: mc_java::Java {
            path: std::path::PathBuf::from("/usr/bin/java"),
            version: mc_java::parse_major("21.0.5").map_or_else(
                || panic!("version lisible"),
                |major| mc_java::Version {
                    full: "21.0.5".into(),
                    major,
                },
            ),
            origin: mc_java::Origin::System,
        },
        neoforge: "21.1.250".into(),
        assets_downloaded: 0,
        libraries: 0,
        client_mods: 42,
        server_mods: 7,
        removed: Vec::new(),
        lock: verrou(vec![entree("jei", "both")]),
        lock_path: atelier.racine.join("samflix.lock.json"),
        previous_lock: None,
        source: "packs/samflix.json".into(),
        from_cache: false,
        ecarts: Vec::new(),
    }
}

/// Le compte rendu dit où le jeu a été posé et combien de mods de chaque côté :
/// c'est ce qu'un joueur relit, et ce qu'il recopie quand il demande de l'aide.
#[test]
fn le_compte_rendu_dit_ou_et_combien() {
    let atelier = Atelier::neuf("install-rendu");
    let rendu = lignes(&resultat(&atelier)).join("\n");

    assert!(rendu.contains("samflix"), "{rendu}");
    assert!(rendu.contains("42 côté client"), "{rendu}");
    assert!(rendu.contains("7 côté serveur"), "{rendu}");
    assert!(rendu.contains("samflix.lock.json"), "{rendu}");
}

/// Une installation hors ligne le dit : le joueur n'a pas le pack publié mais
/// sa copie locale, et c'est la première chose à savoir quand une version
/// attendue manque.
#[test]
fn une_copie_locale_est_annoncee_comme_telle() {
    let atelier = Atelier::neuf("install-cache");
    let mut outcome = resultat(&atelier);

    assert!(!lignes(&outcome).join("\n").contains("hors-ligne"));

    outcome.from_cache = true;
    assert!(lignes(&outcome).join("\n").contains("hors-ligne"));
}

/// Les mods retirés et les changements ne paraissent que s'il y en a. Un titre
/// suivi du vide ferait chercher un changement qui n'a pas eu lieu ; les taire
/// laisserait croire qu'une mise à jour n'a rien fait.
#[test]
fn les_listes_vides_ne_s_annoncent_pas() {
    let atelier = Atelier::neuf("install-listes");
    let mut outcome = resultat(&atelier);

    let sobre = lignes(&outcome).join("\n");
    assert!(!sobre.contains("retirés"), "{sobre}");
    assert!(!sobre.contains("Changements"), "{sobre}");

    outcome.removed = vec!["vieux-mod.jar".into()];
    outcome.previous_lock = Some(verrou(Vec::new()));
    let bavard = lignes(&outcome).join("\n");
    assert!(bavard.contains("vieux-mod.jar"), "{bavard}");
    assert!(bavard.contains("Changements"), "{bavard}");
    assert!(bavard.contains("jei"), "le mod ajouté manque : {bavard}");
}
