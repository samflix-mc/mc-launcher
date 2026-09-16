use super::{Reason, Request, autorite, impasse_implicite};

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

#[test]
fn le_manifeste_fait_autorite_sur_une_dependance_meme_epinglee() {
    let mut epinglee = Request::new("jade");
    epinglee.file = Some("abc".into());

    assert!(
        autorite(&Reason::Explicit, &Request::new("jade"))
            > autorite(&Reason::Declared { by: "x".into() }, &epinglee)
    );
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
