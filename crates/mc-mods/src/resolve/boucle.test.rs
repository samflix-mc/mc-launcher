use super::{resolve, resolve_with};
use crate::essais::{
    Atelier, Projet, Version, jar, jar_avec_deux_embarques, jar_avec_embarque, publier,
};
use crate::jar::Side;
use crate::resolve::demande::Request;
use crate::resolve::registre::Registry;
use crate::resolve::{Options, Plan};

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

/// Un registre dont Modrinth est le serveur de test, et un cache neuf.
fn registre(atelier: &Atelier, serveur: &mc_essais::Serveur) -> Registry {
    Registry::pour_essais(atelier.racine.join("cache"), &serveur.base(), None)
        .expect("le registre se construit")
}

fn demandes(slugs: &[&str]) -> Vec<Request> {
    slugs.iter().map(|s| Request::new(*s)).collect()
}

fn retenus(plan: &Plan) -> Vec<&str> {
    plan.mods
        .iter()
        .map(|m| m.candidate.slug.as_str())
        .collect()
}

/// Le cas nominal, et ce qu'il doit produire : le jar sur le disque, son
/// empreinte recalculée puisque le manifeste n'en donnait pas, et ce que le
/// descripteur du jar déclare fournir.
#[tokio::test]
async fn un_mod_sans_dependance_est_resolu_telecharge_et_lu() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("simple");

    let contenu = jar("jei", &[]);
    serveur.octets("/jei.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("jei").version(Version::nouvelle(
            "19.51.0",
            &serveur.url("/jei.jar"),
            &contenu,
        )),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["jei"]),
        MC,
        LOADER,
    )
    .await
    .expect("la résolution aboutit");

    assert_eq!(retenus(&plan), vec!["jei"]);
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);

    let retenu = &plan.mods[0];
    assert!(retenu.path.is_file(), "{}", retenu.path.display());
    assert!(retenu.provides.contains("jei"));
    // Le cache est indexé par source et par projet : deux mods différents
    // publient parfois un jar au même nom.
    assert!(retenu.path.to_string_lossy().contains("modrinth"));
}

/// Les dépendances saisies par l'auteur au moment de la publication : celles
/// que l'API annonce, et que le résolveur suit sans avoir à ouvrir le jar.
#[tokio::test]
async fn une_dependance_declaree_par_l_api_est_installee() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("declaree");

    let bibliotheque = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &bibliotheque);
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &bibliotheque,
        )),
    );

    let principal = jar("enchantments", &[]);
    serveur.octets("/enchantments.jar", &principal);
    publier(
        &serveur,
        &Projet::nouveau("enchantments").version(
            Version::nouvelle("1.0", &serveur.url("/enchantments.jar"), &principal)
                .declare("bookshelf"),
        ),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retenus(&plan), vec!["bookshelf", "enchantments"]);
}

/// Le cas qui justifie tout le crate : un auteur ajoute une bibliothèque entre
/// deux versions et ne revient pas éditer sa fiche de publication. L'API ne
/// déclare rien ; le jar, lui, l'exige — et sans elle le jeu s'arrête sur
/// « Missing or unsupported mods ».
#[tokio::test]
async fn une_dependance_que_seul_le_jar_declare_est_rattrapee() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("implicite");

    let bibliotheque = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &bibliotheque);
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &bibliotheque,
        )),
    );

    // Le jar exige « bookshelf » ; la version publiée ne le déclare pas.
    let principal = jar("enchantments", &[("bookshelf", "BOTH")]);
    serveur.octets("/enchantments.jar", &principal);
    publier(
        &serveur,
        &Projet::nouveau("enchantments").version(Version::nouvelle(
            "1.0",
            &serveur.url("/enchantments.jar"),
            &principal,
        )),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retenus(&plan), vec!["bookshelf", "enchantments"]);
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);
}

/// Un mod peut embarquer ses bibliothèques par JarJar. Les ignorer ferait
/// conclure à une dépendance manquante et installerait un doublon — deux
/// versions du même mod, ce que NeoForge refuse.
#[tokio::test]
async fn une_bibliotheque_embarquee_ne_provoque_pas_de_doublon() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("jarjar");

    let contenu = jar_avec_embarque("architectury", "cloth-config");
    serveur.octets("/architectury.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("architectury").version(Version::nouvelle(
            "13.0.0",
            &serveur.url("/architectury.jar"),
            &contenu,
        )),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["architectury"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retenus(&plan), vec!["architectury"]);
    // Le jar embarqué est compté comme apport, et non comme identité : c'est
    // ce qui permet à un autre mod d'embarquer la même bibliothèque sans
    // passer pour un doublon.
    assert!(
        plan.mods[0].bundled.contains("cloth-config"),
        "le jar embarqué n'est pas compté : {:?}",
        plan.mods[0].bundled
    );
    assert!(
        !plan.mods[0].provides.contains("cloth-config"),
        "un modId embarqué ne doit pas devenir l'identité du mod"
    );
    assert!(plan.mods[0].fournit().any(|id| id == "cloth-config"));
}

/// Ce que personne ne peut fournir n'arrête pas l'installation : c'est
/// l'appelant qui décide, et il a besoin de savoir quel mod réclamait quoi.
#[tokio::test]
async fn un_modid_introuvable_est_consigne_sans_tout_arreter() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("introuvable");

    let contenu = jar("enchantments", &[("mod-fantome", "BOTH")]);
    serveur.octets("/enchantments.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("enchantments").version(Version::nouvelle(
            "1.0",
            &serveur.url("/enchantments.jar"),
            &contenu,
        )),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retenus(&plan), vec!["enchantments"]);
    assert_eq!(plan.unresolved.len(), 1, "{:?}", plan.unresolved);
    assert_eq!(plan.unresolved[0].mod_id, "mod-fantome");
    assert_eq!(plan.unresolved[0].required_by, "enchantments");
}

/// Désactiver le suivi des dépendances déclarées ne casse rien : le rattrapage
/// par lecture des jars retrouve les mêmes, un tour plus tard. C'est ce qui
/// permet de s'en remettre uniquement à ce que le jeu lira, quand une fiche de
/// publication est fautive.
#[tokio::test]
async fn ignorer_les_dependances_declarees_ne_perd_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("sans-declarees");

    let bibliotheque = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &bibliotheque);
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &bibliotheque,
        )),
    );

    let principal = jar("enchantments", &[("bookshelf", "BOTH")]);
    serveur.octets("/enchantments.jar", &principal);
    publier(
        &serveur,
        &Projet::nouveau("enchantments").version(
            Version::nouvelle("1.0", &serveur.url("/enchantments.jar"), &principal)
                .declare("bookshelf"),
        ),
    );

    let plan = resolve_with(
        &registre(&atelier, &serveur),
        &demandes(&["enchantments"]),
        MC,
        LOADER,
        Options {
            follow_declared: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(retenus(&plan), vec!["bookshelf", "enchantments"]);
}

/// Modrinth publie `client_side` / `server_side` par projet : c'est ce qui
/// donne la répartition sans avoir à la saisir dans le manifeste.
#[tokio::test]
async fn le_cote_publie_par_le_projet_est_retenu() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("cote");

    let contenu = jar("sodium", &[]);
    serveur.octets("/sodium.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("sodium")
            .cote("required", "unsupported")
            .version(Version::nouvelle(
                "0.6",
                &serveur.url("/sodium.jar"),
                &contenu,
            )),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["sodium"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(plan.mods[0].side, Side::Client);
    assert_eq!(plan.for_side(Side::Client).count(), 1);
    assert_eq!(plan.for_side(Side::Server).count(), 0);
}

/// Un mod introuvable dans toutes les sources arrête la résolution : le
/// manifeste le demande explicitement, l'ignorer donnerait un pack incomplet
/// sans que rien ne le dise.
#[tokio::test]
async fn un_mod_du_manifeste_introuvable_arrete_tout() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("absent");

    let erreur = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["mod-qui-n-existe-pas"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("aucune source ne le connaît");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("mod-qui-n-existe-pas"), "{texte}");
    // Sans clé, seule Modrinth a été consultée : le dire évite de chercher
    // pourquoi un mod publié sur CurseForge reste introuvable.
    assert!(texte.contains("aucune clé CurseForge"), "{texte}");
}

/// Le canal par défaut est `release` ; une beta ne doit pas s'inviter dans un
/// pack qui n'en demande pas.
#[tokio::test]
async fn une_beta_n_est_pas_retenue_sans_qu_on_la_demande() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("canal");

    let stable = jar("jei", &[]);
    let beta = jar("jei", &[]);
    serveur.octets("/jei-stable.jar", &stable);
    serveur.octets("/jei-beta.jar", &beta);
    publier(
        &serveur,
        &Projet::nouveau("jei")
            .version(
                Version::nouvelle("19.0.0", &serveur.url("/jei-stable.jar"), &stable)
                    .publie("2026-01-01T00:00:00Z"),
            )
            .version(
                Version::nouvelle("20.0.0-beta", &serveur.url("/jei-beta.jar"), &beta)
                    .canal("beta")
                    .publie("2026-06-01T00:00:00Z"),
            ),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["jei"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(plan.mods[0].candidate.version_number, "19.0.0");
}

/// Deux mods qui se réclament mutuellement sans que la recherche converge : la
/// borne de tours existe pour cela, et son message doit dire ce qui se passe
/// plutôt que de laisser tourner.
#[tokio::test]
async fn un_pack_qui_ne_se_stabilise_pas_finit_par_s_arreter() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("instable");

    // Chaque projet fournit un `modId` différent de celui qu'il exige, et le
    // slug du manquant existe : la file se remplit à chaque tour sans jamais
    // combler l'écart.
    for (rang, suivant) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7)] {
        let slug = format!("chaine{rang}");
        let contenu = jar(
            &format!("fourni{rang}"),
            &[(&format!("chaine{suivant}"), "BOTH")],
        );
        let route = format!("/{slug}.jar");
        serveur.octets(&route, &contenu);
        publier(
            &serveur,
            &Projet::nouveau(&slug).version(Version::nouvelle(
                "1.0",
                &serveur.url(&route),
                &contenu,
            )),
        );
    }

    let erreur = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["chaine0"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("la chaîne ne se referme jamais");

    assert!(
        format!("{erreur:#}").contains("ne se stabilise pas"),
        "{erreur:#}"
    );
}

/// Le cas qui a fait retirer Sodium, Iris et EntityCulling du pack samflix.
///
/// Deux mods distincts embarquent la même bibliothèque — les shims Fabric pour
/// Sodium et Iris, les libs de tr7zw pour EntityCulling et Not Enough
/// Animations. Un `modId` embarqué n'est pas l'identité du mod : NeoForge sait
/// dédupliquer les jars embarqués au chargement, et un pack qui garde Iris sans
/// Sodium est cassé.
#[tokio::test]
async fn deux_mods_qui_embarquent_la_meme_bibliotheque_survivent_tous_les_deux() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("embarque-partage");

    for slug in ["sodium", "iris"] {
        let contenu = jar_avec_embarque(slug, "fabric_api_base");
        let route = format!("/{slug}.jar");
        serveur.octets(&route, &contenu);
        publier(
            &serveur,
            &Projet::nouveau(slug).version(Version::nouvelle(
                "1.0",
                &serveur.url(&route),
                &contenu,
            )),
        );
    }

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["sodium", "iris"]),
        MC,
        LOADER,
    )
    .await
    .expect("les deux mods sont demandés au manifeste");

    assert_eq!(retenus(&plan), vec!["iris", "sodium"], "un mod a disparu");
}

/// Deux jars portant réellement le même `modId` racine restent dédupliqués : la
/// règle d'origine vaut toujours, c'est son périmètre qui était trop large.
/// Ici les deux sont explicites, donc la contradiction du manifeste s'annonce.
#[tokio::test]
async fn deux_mods_explicites_du_meme_modid_racine_arretent_la_resolution() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("meme-modid-racine");

    // Deux projets distincts publient le même mod — le cas d'un fork, ou d'un
    // miroir. NeoForge n'en chargerait qu'un.
    for slug in ["jade", "jade-miroir"] {
        let contenu = jar("jade", &[]);
        let route = format!("/{slug}.jar");
        serveur.octets(&route, &contenu);
        publier(
            &serveur,
            &Projet::nouveau(slug).version(Version::nouvelle(
                "1.0",
                &serveur.url(&route),
                &contenu,
            )),
        );
    }

    let erreur = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["jade", "jade-miroir"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("le manifeste demande deux fois le même mod");

    // Le mod écarté, celui qui garde la place et le modId en cause doivent
    // être nommés : sans eux, le message n'apprend rien.
    let texte = format!("{erreur:#}");
    assert!(texte.contains("jade-miroir"), "{texte}");
    assert!(texte.contains("modId"), "{texte}");
}

/// Une dépendance écartée au profit d'un mod qui porte le même `modId` racine
/// n'est pas une contradiction : personne ne l'avait demandée nommément. La
/// résolution continue, en le disant.
#[tokio::test]
async fn une_dependance_ecartee_par_deduplication_n_arrete_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("dedup-dependance");

    // `principal` exige `jade`, que deux projets fournissent sous le même
    // modId racine ; l'un est demandé au manifeste, l'autre arrive par
    // dépendance déclarée.
    let jade = jar("jade", &[]);
    serveur.octets("/jade.jar", &jade);
    publier(
        &serveur,
        &Projet::nouveau("jade").version(Version::nouvelle(
            "1.0",
            &serveur.url("/jade.jar"),
            &jade,
        )),
    );

    let miroir = jar("jade", &[]);
    serveur.octets("/jade-miroir.jar", &miroir);
    publier(
        &serveur,
        &Projet::nouveau("jade-miroir").version(Version::nouvelle(
            "1.0",
            &serveur.url("/jade-miroir.jar"),
            &miroir,
        )),
    );

    let principal = jar("principal", &[]);
    serveur.octets("/principal.jar", &principal);
    publier(
        &serveur,
        &Projet::nouveau("principal").version(
            Version::nouvelle("1.0", &serveur.url("/principal.jar"), &principal)
                .declare("jade-miroir"),
        ),
    );

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["jade", "principal"]),
        MC,
        LOADER,
    )
    .await
    .expect("une dépendance écartée n'arrête pas l'installation");

    assert_eq!(retenus(&plan), vec!["jade", "principal"]);
}

/// La seconde collision constatée sur le pack : EntityCulling et Not Enough
/// Animations embarquent tous deux `transition` et `trender`, les libs de
/// tr7zw. Deux bibliothèques partagées et non une seule — la déduplication
/// fautive s'arrêtait à la première rencontrée, mais le résultat était le même.
#[tokio::test]
async fn deux_bibliotheques_partagees_ne_font_pas_davantage_un_doublon() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("tr7zw");

    for slug in ["entityculling", "not-enough-animations"] {
        let contenu = jar_avec_deux_embarques(slug, "transition", "trender");
        let route = format!("/{slug}.jar");
        serveur.octets(&route, &contenu);
        publier(
            &serveur,
            &Projet::nouveau(slug).version(Version::nouvelle(
                "1.0",
                &serveur.url(&route),
                &contenu,
            )),
        );
    }

    let plan = resolve(
        &registre(&atelier, &serveur),
        &demandes(&["entityculling", "not-enough-animations"]),
        MC,
        LOADER,
    )
    .await
    .expect("les deux mods sont demandés au manifeste");

    assert_eq!(
        retenus(&plan),
        vec!["entityculling", "not-enough-animations"],
        "un mod a disparu"
    );
}
