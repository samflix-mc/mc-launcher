use super::{Reason, Request, autorite, impasse_implicite};

/// Ces phrases sont la seule explication qu'un joueur reçoive quand un mod
/// qu'il n'a pas demandé apparaît dans son pack, ou qu'un build en remplace un
/// autre. Elles doivent nommer qui a réclamé quoi : une chaîne vide, et le
/// rapport ne dit plus rien.
#[test]
fn chaque_raison_dit_qui_a_reclame_le_mod() {
    assert_eq!(Reason::Explicit.describe(), "demandé par le manifeste");

    let declaree = Reason::Declared {
        by: "create".to_string(),
    };
    assert_eq!(declaree.describe(), "dépendance déclarée de create");

    let implicite = Reason::Implicit {
        by: "create".to_string(),
        mod_id: "flywheel".to_string(),
    };
    assert_eq!(
        implicite.describe(),
        "dépendance implicite : create exige « flywheel »"
    );
}

/// Le scénario qui faisait mourir la résolution sur `MAX_PASSES` : un
/// demandeur autoritaire impose un build qui ne fournit pas le `modId`
/// qu'un jar exige, et l'exigence rejouait sa demande perdue à chaque tour.
#[test]
fn une_exigence_implicite_qui_reperd_sa_place_est_une_impasse() {
    let implicite = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(impasse_implicite(&implicite, 1, 3, false));
}

#[test]
fn une_exigence_implicite_qui_emporte_la_place_n_est_pas_une_impasse() {
    let implicite = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(!impasse_implicite(&implicite, 3, 1, false));
}

/// Perdre l'arbitrage contre un demandeur qui désigne le même jar ne prive
/// de rien : le `modId` sera fourni, la demande n'a plus lieu d'être.
#[test]
fn perdre_contre_le_meme_build_n_est_pas_une_impasse() {
    let implicite = Reason::Implicit {
        by: "build_B".into(),
        mod_id: "libfoo".into(),
    };
    assert!(!impasse_implicite(&implicite, 1, 3, true));
}

/// Une dépendance déclarée écartée est repoussée par son parent quand il
/// est lui-même remplacé ; elle ne se rejoue pas d'elle-même, et n'a donc
/// pas à être retenue comme impasse.
#[test]
fn seules_les_exigences_implicites_font_impasse() {
    let declaree = Reason::Declared { by: "Jade".into() };
    assert!(!impasse_implicite(&declaree, 1, 3, false));
    assert!(!impasse_implicite(&Reason::Explicit, 1, 3, false));
}

/// Le manifeste reste souverain **dès qu'il dit quelque chose** : une demande
/// qu'il épingle bat une dépendance épinglée.
#[test]
fn le_manifeste_fait_autorite_quand_il_epingle_lui_aussi() {
    let mut du_manifeste = Request::new("sodium");
    du_manifeste.file = Some("choix-du-pack".into());
    let mut de_la_dependance = Request::new("sodium");
    de_la_dependance.file = Some("choix-d-iris".into());

    assert!(
        autorite(&Reason::Explicit, &du_manifeste)
            > autorite(&Reason::Declared { by: "iris".into() }, &de_la_dependance)
    );
}

/// L'inverse de ce que faisait l'ancienne règle, et la raison du changement :
/// une demande sans version n'exprime aucune préférence de version. Écrire
/// « sodium » dit « je veux ce mod », pas « je veux sa dernière version quoi
/// qu'il en coûte » — et la laisser écraser l'exigence précise d'Iris faisait
/// tomber Minecraft à la première connexion.
#[test]
fn une_dependance_epinglee_fait_autorite_sur_une_demande_sans_version() {
    let mut de_la_dependance = Request::new("sodium");
    de_la_dependance.file = Some("Pb3OXVqC".into());

    assert!(
        autorite(&Reason::Declared { by: "iris".into() }, &de_la_dependance)
            > autorite(&Reason::Explicit, &Request::new("sodium"))
    );
}

/// Y compris une exigence lue dans un jar, que rien n'annonçait : elle en sait
/// plus sur la version qu'il lui faut qu'une demande qui n'en dit rien.
#[test]
fn toute_demande_epinglee_passe_avant_une_demande_ouverte() {
    let mut epinglee = Request::new("lib");
    epinglee.file = Some("abc".into());
    let implicite = Reason::Implicit {
        by: "create".into(),
        mod_id: "flywheel".into(),
    };

    assert!(autorite(&implicite, &epinglee) > autorite(&Reason::Explicit, &Request::new("lib")));
}

#[test]
fn a_origine_egale_l_epinglage_fait_autorite() {
    let mut epinglee = Request::new("jade");
    epinglee.file = Some("abc".into());
    let raison = Reason::Declared { by: "x".into() };

    assert!(autorite(&raison, &epinglee) > autorite(&raison, &Request::new("jade")));

    // Le numéro de version épingle tout autant que l'identifiant de build.
    let mut par_version = Request::new("jade");
    par_version.version = Some("1.2.3".into());
    assert_eq!(
        autorite(&raison, &par_version),
        autorite(&raison, &epinglee)
    );
}

#[test]
fn une_dependance_declaree_fait_autorite_sur_une_dependance_implicite() {
    assert!(
        autorite(&Reason::Declared { by: "x".into() }, &Request::new("lib"))
            > autorite(
                &Reason::Implicit {
                    by: "x".into(),
                    mod_id: "lib".into()
                },
                &Request::new("lib")
            )
    );
}
