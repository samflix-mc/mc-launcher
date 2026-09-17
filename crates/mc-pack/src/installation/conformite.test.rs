use super::{Pose, ecarts};
use crate::essais::{entree, verrou};
use mc_mods::Origin;

/// `entree` fabrique un mod Modrinth dont le projet est « <slug>-id » et le
/// build « <slug>-1.0 ».
fn pose<'a>(projet: &'a str, build: &'a str) -> Pose<'a> {
    (Origin::Modrinth, projet, build)
}

#[test]
fn un_verrou_rejoue_a_l_identique_ne_signale_rien() {
    let attendu = verrou(vec![entree("jei", "both", None)]);

    assert!(ecarts(&attendu, [pose("jei-id", "jei-1.0")].into_iter()).is_empty());
}

/// La régression qui a rempli le journal d'écarts imaginaires : un mod
/// CurseForge est épinglé sous son slug lisible, et redemandé sous son
/// identifiant numérique. Comparés par slug, c'étaient deux mods différents —
/// un manquant et un en trop — pour un seul mod parfaitement conforme.
#[test]
fn un_projet_demande_par_son_identifiant_reste_le_meme_mod() {
    let mut curseforge = entree("fix-gpu-memory-leak", "both", None);
    curseforge.source = Origin::CurseForge;
    curseforge.project = "882495".to_string();
    curseforge.file = "5513549".to_string();
    let attendu = verrou(vec![curseforge]);

    // Le candidat porte l'identifiant numérique en guise de slug : c'est ce
    // qu'on a demandé, et le projet est pourtant bien le même.
    let ecarts = ecarts(
        &attendu,
        [(Origin::CurseForge, "882495", "5513549")].into_iter(),
    );

    assert!(ecarts.is_empty(), "{ecarts:?}");
}

#[test]
fn un_mod_du_verrou_qui_manque_est_nomme() {
    // Une source qui a retiré un build, une résolution qui n'a rien trouvé :
    // l'installation se termine « bien », et NeoForge refusera de démarrer.
    let attendu = verrou(vec![
        entree("jei", "both", None),
        entree("jade", "both", None),
    ]);

    let ecarts = ecarts(&attendu, [pose("jei-id", "jei-1.0")].into_iter());

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    // Nommé par son slug, qui est ce qu'un humain reconnaît.
    assert!(ecarts[0].contains("jade"), "{ecarts:?}");
    assert!(ecarts[0].contains("absent"), "{ecarts:?}");
}

#[test]
fn un_build_qui_a_glisse_est_nomme_avec_les_deux_versions() {
    // Même projet, autre build : c'est ce qui fait diverger un client de son
    // serveur, et le seul écart qu'une comparaison par projet manquerait si
    // elle ne regardait pas aussi le build.
    let attendu = verrou(vec![entree("jei", "both", None)]);

    let ecarts = ecarts(&attendu, [pose("jei-id", "jei-2.0")].into_iter());

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("jei-1.0"), "{ecarts:?}");
    assert!(ecarts[0].contains("jei-2.0"), "{ecarts:?}");
}

#[test]
fn un_mod_en_trop_compte_autant_qu_un_mod_en_moins() {
    // NeoForge négocie ses registres à la connexion : un jar en trop fait
    // échouer la négociation aussi sûrement qu'un jar en moins.
    let attendu = verrou(vec![entree("jei", "both", None)]);

    let ecarts = ecarts(
        &attendu,
        [pose("jei-id", "jei-1.0"), pose("sodium-id", "sodium-1.0")].into_iter(),
    );

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("sodium-id"), "{ecarts:?}");
    assert!(ecarts[0].contains("absent du verrou"), "{ecarts:?}");
}

/// Deux sources peuvent porter le même identifiant de projet sans que ce soit
/// le même mod : la clé est le couple, pas l'identifiant seul.
#[test]
fn deux_sources_ne_se_confondent_pas_sur_un_identifiant_commun() {
    let attendu = verrou(vec![entree("jei", "both", None)]);

    let ecarts = ecarts(
        &attendu,
        [(Origin::CurseForge, "jei-id", "jei-1.0")].into_iter(),
    );

    assert_eq!(ecarts.len(), 2, "{ecarts:?}");
}

#[test]
fn un_verrou_vide_et_un_plan_vide_coincident() {
    let vide: [Pose<'_>; 0] = [];

    assert!(ecarts(&verrou(Vec::new()), vide.into_iter()).is_empty());
}

/// Le cas réel qui a fait tomber le jeu : le manifeste demande « sodium » sans
/// version et obtient le dernier build (0.8.13), pendant qu'Iris exige
/// explicitement 0.6.13 pour ses mixins de compatibilité. Le pack s'installe,
/// et Minecraft tombe à la première connexion sur une classe disparue.
#[test]
fn une_dependance_epinglee_non_satisfaite_est_nommee() {
    let poses = [
        (Origin::Modrinth, "AANobbMI", "uMOpc5uV"), // sodium 0.8.13
        (Origin::Modrinth, "YL57xq9U", "t3ruzodq"), // iris 1.8.12
    ];

    let ecarts = super::dependances_insatisfaites(
        &poses,
        [super::Exigence {
            par: "iris",
            origin: Origin::Modrinth,
            projet: "AANobbMI",
            build: "Pb3OXVqC", // sodium 0.6.13
        }]
        .into_iter(),
    );

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("iris"), "{ecarts:?}");
    assert!(ecarts[0].contains("Pb3OXVqC"), "{ecarts:?}");
    assert!(ecarts[0].contains("uMOpc5uV"), "{ecarts:?}");
}

#[test]
fn une_dependance_satisfaite_ne_dit_rien() {
    let poses = [(Origin::Modrinth, "AANobbMI", "Pb3OXVqC")];

    let ecarts = super::dependances_insatisfaites(
        &poses,
        [super::Exigence {
            par: "iris",
            origin: Origin::Modrinth,
            projet: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(ecarts.is_empty(), "{ecarts:?}");
}

/// Une dépendance absente du pack relève de `Plan::unresolved`, qui la nomme
/// déjà. La compter ici la ferait apparaître deux fois.
#[test]
fn une_dependance_absente_du_pack_n_est_pas_notre_affaire() {
    let poses: [Pose<'_>; 0] = [];

    let ecarts = super::dependances_insatisfaites(
        &poses,
        [super::Exigence {
            par: "iris",
            origin: Origin::Modrinth,
            projet: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(ecarts.is_empty(), "{ecarts:?}");
}

/// Une exigence chez une source, le mod posé chez l'autre : ce n'est pas le
/// même projet, et les confondre inventerait un conflit.
#[test]
fn une_exigence_ne_traverse_pas_les_sources() {
    let poses = [(Origin::CurseForge, "AANobbMI", "autre")];

    let ecarts = super::dependances_insatisfaites(
        &poses,
        [super::Exigence {
            par: "iris",
            origin: Origin::Modrinth,
            projet: "AANobbMI",
            build: "Pb3OXVqC",
        }]
        .into_iter(),
    );

    assert!(ecarts.is_empty(), "{ecarts:?}");
}
