//! La pose, éprouvée dans son propre processus.
//!
//! `poser` écrit un `OnceLock` : il n'y a qu'une pose par processus, et les
//! tests d'un même binaire s'exécutent en parallèle dans des fils. Un second
//! test qui poserait le sien échouerait ou réussirait selon l'ordonnanceur.
//!
//! D'où ce binaire séparé, qui ne contient qu'UN test.

use std::path::PathBuf;

use mc_chemins::{Emplacements, courants, depuis_bases, poser, poses};

/// Ce que ce test tue, et qu'aucun autre ne pourrait tuer : le mutant qui
/// remplace le corps de `courants()` par `du_systeme()`.
///
/// Il faut pour cela que les racines posées soient IMPOSSIBLES à confondre
/// avec celles du système. D'où `/tmp/mc-chemins-<pid>/…` : un chemin
/// artificiel, qu'aucune convention de plateforme ne produirait. Poser les
/// mêmes racines que le système ferait passer le test avec le mutant en place,
/// et le survivant vivrait indéfiniment.
#[test]
fn une_pose_s_impose_puis_ne_se_remplace_plus() {
    let artificielle = PathBuf::from(format!("/tmp/mc-chemins-{}", std::process::id()));

    assert!(!poses(), "rien ne doit avoir été posé avant ce test");
    let avant = courants();

    let voulus = depuis_bases(mc_chemins::Bases {
        donnees: artificielle.join("donnees"),
        config: artificielle.join("config"),
        temporaire: artificielle.join("tmp"),
    });

    poser(voulus.clone()).expect("la première pose réussit");
    assert!(poses());

    // Ce qu'on lit est ce qu'on a posé, et non ce que l'environnement dit.
    assert_eq!(courants(), voulus);
    assert_ne!(courants(), avant, "la pose n'a rien changé");
    assert!(courants().donnees.starts_with(&artificielle));
    // Et la déduction s'applique aussi à ce qu'on impose : les journaux
    // restent sous les données.
    assert_eq!(courants().journaux, voulus.donnees.join("logs"));

    // Une seconde pose est refusée, et ne remplace rien. C'est un bogue de
    // séquencement qu'il faut voir, pas une situation à rattraper : deux
    // moitiés du programme sur deux arborescences seraient pires que l'une des
    // deux, quelle qu'elle soit.
    let autres = Emplacements {
        donnees: PathBuf::from("/nulle/part"),
        config: PathBuf::from("/nulle/part"),
        journaux: PathBuf::from("/nulle/part"),
        temporaire: PathBuf::from("/nulle/part"),
    };
    poser(autres).expect_err("la seconde pose est refusée");
    assert_eq!(courants(), voulus, "la seconde pose a remplacé la première");
}
