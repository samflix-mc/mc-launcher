use super::{Channel, Request, check_compatible, pick};
use crate::resolve::essais::installed;

/// Un build épinglé échappe au filtrage des API : personne d'autre ne vérifie
/// qu'il vise bien le chargeur du pack. Poser un jar Fabric dans un pack
/// NeoForge donne un jeu qui ne démarre pas, et un message qui ne dit pas
/// pourquoi.
#[test]
fn un_build_epingle_pour_un_autre_chargeur_est_refuse() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.file_name = "jei-fabric-1.21.1-19.21.jar".into();

    let refus = check_compatible(&candidate, "1.21.1", "neoforge", "jei")
        .expect_err("un jar Fabric n'a rien à faire dans un pack NeoForge");
    assert!(
        refus.to_string().contains("autre chargeur"),
        "message inattendu : {refus}"
    );
}

/// Beaucoup de mods publient un jar par chargeur sous un nom qui les cite
/// tous. Le refuser sur la seule présence du mot « fabric » écarterait des
/// builds parfaitement valables — c'est l'absence du nôtre qui condamne.
#[test]
fn un_nom_qui_cite_les_deux_chargeurs_passe() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.file_name = "jei-fabric-neoforge-1.21.1.jar".into();
    assert!(check_compatible(&candidate, "1.21.1", "neoforge", "jei").is_ok());

    // Et un nom qui ne cite aucun chargeur ne se juge pas : le nom de fichier
    // est trop peu fiable pour rejeter sur un silence.
    let mut muet = installed("jei", &["jei"], &[]).candidate;
    muet.file_name = "jei-19.21.jar".into();
    assert!(check_compatible(&muet, "1.21.1", "neoforge", "jei").is_ok());
}

/// L'épinglage se fait sous le nom qu'on a sous les yeux, et celui-ci n'est pas
/// le même selon l'API : Modrinth affiche un numéro de version, CurseForge un
/// nom de fichier, la page du projet un titre. Les trois doivent retrouver le
/// même build.
#[test]
fn une_version_epinglee_se_reconnait_sous_ses_trois_noms() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.version_number = "19.21.0.247".into();
    candidate.file_name = "jei-1.21.1-neoforge-19.21.0.247.jar".into();
    candidate.display_name = "Just Enough Items 19.21".into();

    for nom in [
        "19.21.0.247",
        "jei-1.21.1-neoforge-19.21.0.247.jar",
        "Just Enough Items 19.21",
        // La casse ne doit pas décider : elle varie d'une page à l'autre.
        "JEI-1.21.1-NEOFORGE-19.21.0.247.JAR",
    ] {
        let mut request = Request::new("jei");
        request.version = Some(nom.to_string());
        assert!(
            pick(vec![candidate.clone()], &request).is_some(),
            "épinglage par « {nom} » non reconnu"
        );
    }
}

#[test]
fn le_canal_le_plus_recent_est_retenu() {
    let mut vieux = installed("jei", &["jei"], &[]).candidate;
    vieux.published = "2024-01-01".into();
    let mut recent = vieux.clone();
    recent.published = "2025-06-01".into();
    recent.version_number = "19.56".into();

    let choix = pick(vec![vieux, recent], &Request::new("jei")).unwrap();
    assert_eq!(choix.version_number, "19.56");
}

#[test]
fn une_beta_est_ecartee_par_defaut() {
    let mut beta = installed("jei", &["jei"], &[]).candidate;
    beta.channel = Channel::Beta;
    assert!(pick(vec![beta.clone()], &Request::new("jei")).is_none());

    let mut request = Request::new("jei");
    request.channel = Some(Channel::Beta);
    assert!(pick(vec![beta], &request).is_some());
}

#[test]
fn une_version_epinglee_qui_n_existe_pas_ne_retombe_sur_rien() {
    // Silencieusement retomber sur une autre version annulerait tout
    // l'intérêt de l'épinglage.
    let candidate = installed("jei", &["jei"], &[]).candidate;
    let mut request = Request::new("jei");
    request.version = Some("99.99".into());
    assert!(pick(vec![candidate], &request).is_none());
}
