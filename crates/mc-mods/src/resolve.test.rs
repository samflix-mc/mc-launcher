use super::*;
use crate::jar::Requirement;

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

fn candidat(slug: &str, version: &str) -> Candidate {
    Candidate {
        origin: Origin::Modrinth,
        project_id: format!("{slug}-id"),
        slug: slug.to_string(),
        name: slug.to_string(),
        version_id: format!("{slug}-{version}"),
        version_number: version.to_string(),
        display_name: version.to_string(),
        channel: Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        published: "2025-01-01".into(),
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: Some(format!("https://modrinth.com/mod/{slug}")),
        redistributable: true,
    }
}

/// Sans clé CurseForge, la moitié des sources n'a pas été interrogée : le
/// dire évite de chercher pourquoi un mod « n'existe pas ».
#[test]
fn projet_introuvable_signale_la_source_non_consultee() {
    let erreur = trancher(
        Vec::new(),
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        false,
    )
    .expect_err("aucun candidat");
    let texte = erreur.to_string();
    assert!(
        texte.contains("introuvable pour Minecraft 1.21.1 / neoforge"),
        "{texte}"
    );
    assert!(
        texte.contains("aucune clé CurseForge configurée"),
        "{texte}"
    );

    let avec_cle = trancher(
        Vec::new(),
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("aucun candidat");
    assert!(!avec_cle.to_string().contains("CurseForge configurée"));
}

/// Le projet existe, la version demandée non : deux situations qu'un même
/// message confondrait, alors qu'elles n'appellent pas la même correction.
#[test]
fn version_demandee_absente_le_dit_plutot_que_introuvable() {
    let mut request = Request::new("jade");
    request.version = Some("99.0.0".into());
    let erreur = trancher(
        vec![candidat("jade", "15.10.6")],
        &request,
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("version absente");
    let texte = erreur.to_string();
    assert!(texte.contains("aucune version ne correspond"), "{texte}");
    assert!(texte.contains("« 99.0.0 »"), "{texte}");
}

#[test]
fn canal_trop_strict_nomme_le_canal() {
    let mut candidats = vec![candidat("jade", "15.10.6")];
    candidats[0].channel = Channel::Beta;
    let mut request = Request::new("jade");
    request.channel = Some(Channel::Release);
    let erreur =
        trancher(candidats, &request, "1.21.1", "neoforge", true).expect_err("aucune release");
    assert!(erreur.to_string().contains("au canal release"), "{erreur}");
}

/// Un auteur peut interdire le téléchargement par un tiers. Ce n'est pas
/// une panne : le fichier existe, il faut aller le chercher à la main, et
/// le message doit dire où.
#[test]
fn telechargement_interdit_renvoie_vers_la_page_du_mod() {
    let mut interdit = candidat("jade", "15.10.6");
    interdit.redistributable = false;
    let erreur = trancher(
        vec![interdit],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("non redistribuable");
    let texte = erreur.to_string();
    assert!(texte.contains("apports manuels"), "{texte}");
    assert!(texte.contains("https://modrinth.com/mod/jade"), "{texte}");
}

/// Une URL vide vaut un téléchargement impossible : la distinction ne se
/// voit qu'au moment de télécharger, trop tard pour l'expliquer.
#[test]
fn url_vide_vaut_telechargement_interdit() {
    let mut sans_url = candidat("jade", "15.10.6");
    sans_url.url = String::new();
    let erreur = trancher(
        vec![sans_url],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("url vide");
    assert!(erreur.to_string().contains("apports manuels"));
}

#[test]
fn le_build_retenu_est_rendu_tel_quel() {
    let retenu = trancher(
        vec![candidat("jade", "15.10.6")],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect("un build retenu");
    assert_eq!(retenu.version_number, "15.10.6");
}

/// Le cas nominal du rattrapage : un jar exige un modId que personne n'a
/// déclaré, on trouve qui le fournit, et sa demande rejoint la file.
#[test]
fn un_fournisseur_trouve_devient_une_demande_implicite() {
    let suite = suite_du_rattrapage(
        Some(candidat("bookshelf-lib", "21.1.81")),
        &BTreeSet::new(),
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    match suite {
        Rattrapage::Demander(request, reason) => {
            assert_eq!(request.slug, "bookshelf-lib-id");
            assert_eq!(request.source, Some(Origin::Modrinth));
            assert_eq!(request.file.as_deref(), Some("bookshelf-lib-21.1.81"));
            assert_eq!(
                reason,
                Reason::Implicit {
                    by: "attributefix".into(),
                    mod_id: "bookshelf".into()
                }
            );
        }
        autre => panic!("attendu une demande, obtenu {autre:?}"),
    }
}

/// Le cas qui faisait tourner la résolution jusqu'à MAX_PASSES : le seul
/// fournisseur occupe une clé qu'une demande plus autoritaire tient déjà.
/// Redemander ne changerait rien, donc on consigne le manque.
#[test]
fn un_fournisseur_en_impasse_est_consigne_au_lieu_d_etre_redemande() {
    let mut impasses = BTreeSet::new();
    impasses.insert((Origin::Modrinth, "bookshelf-lib-id".to_string()));
    let suite = suite_du_rattrapage(
        Some(candidat("bookshelf-lib", "21.1.81")),
        &impasses,
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    assert_eq!(
        suite,
        Rattrapage::Renoncer(Unresolved {
            mod_id: "bookshelf".into(),
            required_by: "attributefix".into(),
            side: Side::Both,
        })
    );
}

#[test]
fn un_modid_que_personne_ne_fournit_est_consigne() {
    let suite = suite_du_rattrapage(
        None,
        &BTreeSet::new(),
        "libfoo".into(),
        "build_b".into(),
        Side::Client,
    );
    assert_eq!(
        suite,
        Rattrapage::Renoncer(Unresolved {
            mod_id: "libfoo".into(),
            required_by: "build_b".into(),
            side: Side::Client,
        })
    );
}

fn installed(slug: &str, provides: &[&str], requires: &[(&str, Side)]) -> Installed {
    Installed {
        candidate: Candidate {
            origin: Origin::Modrinth,
            project_id: slug.to_string(),
            slug: slug.to_string(),
            name: slug.to_string(),
            version_id: "v".into(),
            version_number: "1.0".into(),
            display_name: "1.0".into(),
            channel: Channel::Release,
            file_name: format!("{slug}.jar"),
            url: format!("https://exemple/{slug}.jar"),
            sha1: None,
            sha512: None,
            size: 0,
            published: "2025-01-01".into(),
            project_side: Side::Both,
            declared_deps: Vec::new(),
            page_url: None,
            redistributable: true,
        },
        side: Side::Both,
        reason: Reason::Explicit,
        autorite: 4,
        path: PathBuf::from("/cache").join(format!("{slug}.jar")),
        provides: provides.iter().map(|s| s.to_string()).collect(),
        requires: requires
            .iter()
            .map(|(id, side)| Requirement {
                mod_id: id.to_string(),
                version_range: None,
                side: *side,
            })
            .collect(),
    }
}

fn map(entries: Vec<Installed>) -> BTreeMap<(Origin, String), Installed> {
    entries
        .into_iter()
        .map(|e| ((e.candidate.origin, e.candidate.project_id.clone()), e))
        .collect()
}

#[test]
fn une_dependance_absente_est_signalee() {
    let chosen = map(vec![installed(
        "attributefix",
        &["attributefix"],
        &[("bookshelf", Side::Both)],
    )]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "bookshelf");
    assert_eq!(missing[0].1, "attributefix");
}

#[test]
fn une_dependance_fournie_sous_un_autre_slug_ne_manque_pas() {
    // Le modId `bookshelf` est publié sous le slug `bookshelf-lib` : c'est
    // le modId du jar qui fait foi, jamais le nom du projet.
    let chosen = map(vec![
        installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        ),
        installed("bookshelf-lib", &["bookshelf"], &[]),
    ]);
    assert!(missing_requirements(&chosen).is_empty());
}

#[test]
fn une_dependance_embarquee_par_jarjar_ne_manque_pas() {
    // Le mod embarque sa bibliothèque : l'installer en plus donnerait deux
    // versions du même modId, ce que NeoForge refuse au chargement.
    let chosen = map(vec![installed(
        "un-mod",
        &["unmod", "unelib"],
        &[("unelib", Side::Both)],
    )]);
    assert!(missing_requirements(&chosen).is_empty());
}

#[test]
fn les_cotes_de_deux_demandeurs_sont_fusionnes() {
    let chosen = map(vec![
        installed("a", &["a"], &[("lib", Side::Client)]),
        installed("b", &["b"], &[("lib", Side::Server)]),
    ]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].2, Side::Both);
}

#[test]
fn le_plan_filtre_par_cote() {
    let mut client_only = installed("embeddium", &["embeddium"], &[]);
    client_only.side = Side::Client;
    let plan = Plan {
        mods: vec![installed("jei", &["jei"], &[]), client_only],
        unresolved: Vec::new(),
    };
    assert_eq!(plan.for_side(Side::Server).count(), 1);
    assert_eq!(plan.for_side(Side::Client).count(), 2);
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
fn un_meme_mod_venu_de_deux_sources_n_est_garde_qu_une_fois() {
    // Demandé par son slug Modrinth, puis tiré comme dépendance par son
    // identifiant CurseForge : rien ne rapproche les deux clés de projet,
    // sauf le modId. Deux jars du même modId feraient échouer NeoForge.
    let mut depuis_modrinth = installed("jade", &["jade"], &[]);
    depuis_modrinth.candidate.sha1 = Some("aa".into());

    let mut depuis_cf = installed("jade-cf", &["jade"], &[]);
    depuis_cf.candidate.origin = Origin::CurseForge;
    depuis_cf.candidate.sha1 = None;
    depuis_cf.reason = Reason::Declared { by: "autre".into() };

    let mut chosen = map(vec![depuis_modrinth, depuis_cf]);
    deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    // Celui qui porte une empreinte est conservé : il est vérifiable.
    assert!(chosen.values().next().unwrap().candidate.sha1.is_some());
}

#[test]
fn a_empreinte_egale_le_mod_demande_l_emporte() {
    let mut explicite = installed("jade", &["jade"], &[]);
    explicite.candidate.sha1 = Some("aa".into());

    let mut dependance = installed("jade-bis", &["jade"], &[]);
    dependance.candidate.sha1 = Some("bb".into());
    dependance.reason = Reason::Declared { by: "autre".into() };

    let mut chosen = map(vec![explicite, dependance]);
    deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen.values().next().unwrap().reason, Reason::Explicit);
}

#[test]
fn deux_mods_distincts_ne_sont_pas_deduplicates() {
    let mut chosen = map(vec![
        installed("jei", &["jei"], &[]),
        installed("jade", &["jade"], &[]),
    ]);
    deduplicate_by_mod_id(&mut chosen);
    assert_eq!(chosen.len(), 2);
}

fn cle(source: Origin, projet: &str) -> Cle {
    (source, projet.to_string())
}

#[test]
fn le_manifeste_passe_avant_les_dependances_meme_poussees_en_cours_de_route() {
    // Le défaut d'origine tenait entièrement ici. En pile unique, la
    // dépendance poussée par « b » passait avant la demande « a » restante :
    // « a » était résolu sans son épinglage, et la demande épinglée trouvait
    // ensuite la clé prise. Le joueur installait un autre build que celui du
    // verrou, sans que rien ne le signale.
    let mut queue = FileDeResolution::default();
    queue.pousser(Request::new("a"), Reason::Explicit, None);
    queue.pousser(Request::new("b"), Reason::Explicit, None);

    let premier = queue.suivante().unwrap();
    assert_eq!(premier.request.slug, "b");
    queue.pousser(
        Request::new("a"),
        Reason::Declared { by: "b".into() },
        Some(cle(Origin::Modrinth, "b")),
    );

    let ensuite = queue.suivante().unwrap();
    assert_eq!(ensuite.request.slug, "a");
    assert_eq!(ensuite.reason, Reason::Explicit);

    let enfin = queue.suivante().unwrap();
    assert_eq!(enfin.request.slug, "a");
    assert_eq!(enfin.reason, Reason::Declared { by: "b".into() });
    assert!(queue.est_vide());
}

#[test]
fn remplacer_un_build_oublie_les_dependances_de_celui_qu_on_ecarte() {
    // Sinon on installe les bibliothèques de la version écartée en plus de
    // celles de la version retenue, et le verrou les consigne comme
    // dépendances d'un build qui n'est pas là.
    let mut queue = FileDeResolution::default();
    let x = cle(Origin::Modrinth, "X");
    let y = cle(Origin::Modrinth, "Y");
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.pousser(
        Request::new("libB"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.pousser(
        Request::new("libC"),
        Reason::Declared { by: "Y".into() },
        Some(y),
    );

    queue.oublier_dependances_de(&x);

    // Celles d'un autre demandeur restent : Y n'a pas été remplacé.
    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libC");
    assert!(queue.est_vide());
}

#[test]
fn oublier_les_dependances_epargne_l_homonyme_venu_de_l_autre_source() {
    // « Jade » se publie sous le même titre chez les deux sources, et les
    // deux projets coexistent jusqu'à la déduplication finale. Retirer les
    // dépendances par le titre emportait celles du jumeau, que plus rien ne
    // repoussait : le pack partait sans sa bibliothèque.
    let mut queue = FileDeResolution::default();
    let modrinth = cle(Origin::Modrinth, "nvQzSEkR");
    let curseforge = cle(Origin::CurseForge, "324717");
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "Jade".into() },
        Some(modrinth.clone()),
    );
    queue.pousser(
        Request::new("libB"),
        Reason::Declared { by: "Jade".into() },
        Some(curseforge),
    );

    queue.oublier_dependances_de(&modrinth);

    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libB");
    assert!(queue.est_vide());
}

#[test]
fn oublier_les_dependances_epargne_le_manifeste() {
    // Un mod du manifeste qui porte le nom d'un parent remplacé n'a pas à
    // disparaître : sa demande ne vient pas de ce parent.
    let mut queue = FileDeResolution::default();
    let x = cle(Origin::Modrinth, "X");
    queue.pousser(Request::new("libA"), Reason::Explicit, None);
    queue.pousser(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );

    queue.oublier_dependances_de(&x);

    let reste = queue.suivante().unwrap();
    assert_eq!(reste.request.slug, "libA");
    assert_eq!(reste.reason, Reason::Explicit);
    assert!(queue.est_vide());
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

#[test]
fn une_version_epinglee_qui_n_existe_pas_ne_retombe_sur_rien() {
    // Silencieusement retomber sur une autre version annulerait tout
    // l'intérêt de l'épinglage.
    let candidate = installed("jei", &["jei"], &[]).candidate;
    let mut request = Request::new("jei");
    request.version = Some("99.99".into());
    assert!(pick(vec![candidate], &request).is_none());
}
