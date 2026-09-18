use super::super::{LockedLoader, Lockfile};
use super::tests::{lock, locked};
use crate::essais::{Atelier, entree, verrou};
use mc_mods::Origin;

/// Ce qui est écrit doit se relire à l'identique : le verrou est ce qui permet
/// de rejouer une installation des mois plus tard, quand toutes les versions
/// ont bougé.
#[test]
fn un_verrou_ecrit_se_relit_a_l_identique() {
    let atelier = Atelier::neuf("verrou-aller-retour");
    let chemin = atelier.racine.join("samflix.lock.json");

    let mut ecrit = verrou(vec![entree("jei", "both", Some(b"jei"))]);
    ecrit
        .unresolved
        .push(crate::essais::manque("fantome", "jei"));
    ecrit.save(&chemin).expect("écriture");

    let relu = Lockfile::load(&chemin).expect("lecture");
    assert_eq!(relu.name, "samflix");
    assert_eq!(relu.mods.len(), 1);
    assert_eq!(relu.mods[0].file_name, "jei.jar");
    assert_eq!(relu.mods[0].sha1, ecrit.mods[0].sha1);
    assert_eq!(relu.unresolved.len(), 1);
    assert_eq!(relu.unresolved[0].mod_id, "fantome");
}

/// Le verrou se versionne à côté du manifeste : un fichier qui se termine par
/// une ligne vide se relit mieux dans une revue.
#[test]
fn le_verrou_ecrit_se_termine_par_une_ligne() {
    let atelier = Atelier::neuf("verrou-ligne");
    let chemin = atelier.racine.join("samflix.lock.json");
    verrou(Vec::new()).save(&chemin).unwrap();

    let texte = std::fs::read_to_string(&chemin).unwrap();
    assert!(texte.ends_with("}\n"), "{:?}", &texte[texte.len() - 5..]);
}

/// Un verrou absent ou illisible doit se dire avec son chemin : c'est un
/// fichier qu'on va aller regarder.
#[test]
fn un_verrou_absent_ou_casse_nomme_son_chemin() {
    let atelier = Atelier::neuf("verrou-casse");

    let absent = atelier.racine.join("nulle-part.lock.json");
    let erreur = Lockfile::load(&absent).expect_err("rien à cette place");
    assert!(format!("{erreur:#}").contains("nulle-part"), "{erreur:#}");

    let casse = atelier.ecrire("casse.lock.json", b"{ceci n'est pas du JSON");
    let erreur = Lockfile::load(&casse).expect_err("JSON invalide");
    assert!(format!("{erreur:#}").contains("illisible"), "{erreur:#}");
}

/// Les verrous écrits avant que `sha512` et `url` n'existent restent lisibles :
/// refuser de les lire empêcherait de jouer sans rien apporter.
#[test]
fn un_verrou_d_avant_les_derniers_champs_reste_lisible() {
    let brut = r#"{"schema":1,"pack":"samflix","generated":"2025-01-01T00:00:00Z",
      "minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,
      "mods":[{"slug":"jei","name":"JEI","origin":"modrinth","project":"p","file":"f",
               "version":"1.0","file_name":"jei.jar","sha1":"aa","size":1,"side":"both",
               "reason":"demandé","provides":["jei"]}]}"#
        .as_bytes();

    let relu = Lockfile::parse(brut).expect("verrou hérité");
    assert!(relu.mods[0].url.is_empty());
    assert!(relu.mods[0].sha512.is_none());
    // Leur SHA-1 continue de faire foi jusqu'au prochain « lock ».
    assert_eq!(
        relu.mods[0].checksum(),
        Some(mc_dl::Checksum::Sha1("aa".into()))
    );
    assert!(relu.unresolved.is_empty());
}

/// Le verrou naît d'un plan de résolution : c'est là que se fige ce qui a été
/// réellement installé, et pourquoi.
#[test]
fn un_plan_de_resolution_devient_un_verrou() {
    use mc_mods::{Plan, Side};

    let plan = Plan {
        mods: Vec::new(),
        unresolved: vec![mc_mods::resolve::Unresolved {
            mod_id: "fantome".into(),
            required_by: "jei".into(),
            side: Side::Both,
        }],
    };

    let manifeste = crate::manifest::Manifest::parse(crate::essais::MANIFESTE.as_bytes())
        .expect("le manifeste d'essai se lit");

    let lock = Lockfile::from_plan(
        &manifeste,
        crate::lockfile::LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        21,
        &plan,
    );

    assert_eq!(lock.name, "samflix");
    assert_eq!(lock.minecraft, "1.21.1");
    assert_eq!(lock.java, 21);
    assert_eq!(lock.unresolved.len(), 1);
    assert_eq!(lock.unresolved[0].mod_id, "fantome");
    // La date de génération est posée à l'écriture, pas devinée.
    assert!(lock.generated.ends_with('Z'), "{}", lock.generated);
}

/// Un manifeste sans extension ne doit pas produire un verrou sans nom.
#[test]
fn un_manifeste_sans_extension_a_quand_meme_un_verrou() {
    assert_eq!(
        Lockfile::path_for(std::path::Path::new("packs/samflix")),
        std::path::Path::new("packs/samflix.lock.json")
    );
}

/// Les verrous déjà publiés emploient `pack` et `origin`. Ils doivent
/// continuer de se lire : le fichier distant est téléchargé à chaque
/// installation, et refuser de le lire empêcherait de jouer jusqu'à ce que
/// quelqu'un le republie.
#[test]
fn un_verrou_d_avant_le_renommage_se_lit_encore() {
    let ancien = r#"{
        "schema": 1,
        "pack": "samflix",
        "generated": "2026-09-17T00:00:00Z",
        "minecraft": "1.21.1",
        "loader": {"type": "neoforge", "version": "21.1.250"},
        "java": 21,
        "mods": [{
            "slug": "jei",
            "name": "Just Enough Items",
            "origin": "modrinth",
            "project": "u6dRKJwZ",
            "file": "abc",
            "version": "19.51.0",
            "file_name": "jei.jar",
            "size": 1,
            "side": "both",
            "reason": "demandé par le manifeste",
            "provides": ["jei"]
        }]
    }"#;

    let relu = Lockfile::parse(ancien.as_bytes()).expect("un verrou d'avant reste lisible");

    assert_eq!(relu.name, "samflix");
    assert_eq!(relu.mods[0].source, Origin::Modrinth);
    // Le canal manquait : on ne suppose pas une préversion sur un pack que
    // personne n'a touché.
    assert_eq!(relu.mods[0].channel, mc_mods::Channel::Release);
    // Et les champs neufs sont simplement absents.
    assert_eq!(relu.version, None);
    assert!(relu.servers.is_empty());
}

/// Ce qu'on écrit désormais emploie les noms du manifeste, et rien d'autre :
/// les deux fichiers se lisent l'un après l'autre.
#[test]
fn le_verrou_ecrit_emploie_le_vocabulaire_du_manifeste() {
    let ecrit =
        serde_json::to_string(&lock(vec![locked("jei", "abc", "19.51.0")])).expect("sérialisation");

    assert!(ecrit.contains("\"name\":"), "{ecrit}");
    assert!(ecrit.contains("\"source\":"), "{ecrit}");
    assert!(!ecrit.contains("\"pack\":"), "{ecrit}");
    assert!(!ecrit.contains("\"origin\":"), "{ecrit}");
}

/// Un verrou seul doit suffire : il porte la version du pack et les serveurs,
/// que seul le manifeste donnait. Un script de serveur n'a plus besoin des
/// deux fichiers.
#[test]
fn le_verrou_reprend_ce_que_le_manifeste_declare() {
    let manifeste = crate::manifest::Manifest::parse(crate::essais::MANIFESTE.as_bytes())
        .expect("le manifeste d'essai se lit");

    let verrou = Lockfile::from_plan(
        &manifeste,
        LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        21,
        &mc_mods::Plan {
            mods: Vec::new(),
            unresolved: Vec::new(),
        },
    );

    assert_eq!(verrou.name, manifeste.name);
    assert_eq!(verrou.version, manifeste.version);
    assert_eq!(
        verrou.servers.keys().collect::<Vec<_>>(),
        manifeste.servers.keys().collect::<Vec<_>>()
    );
    assert_eq!(verrou.minecraft, manifeste.minecraft);
}

/// Les verrous publiés avant que `generation` n'existe se lisent en
/// génération 0. Refuser de les lire empêcherait de jouer jusqu'à ce que
/// quelqu'un les republie — exactement la panne qu'un mécanisme de
/// réinstallation forcée ne doit pas causer.
#[test]
fn un_verrou_d_avant_la_generation_se_lit_en_zero() {
    let ancien = r#"{"schema":1,"name":"samflix","generated":"2026-09-17T00:00:00Z",
      "minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,
      "mods":[]}"#;

    let relu = Lockfile::parse(ancien.as_bytes()).expect("un verrou d'avant reste lisible");
    assert_eq!(relu.generation, 0);
}

/// Et la génération s'écrit TOUJOURS, même à zéro : pas de
/// `skip_serializing_if`. Sans cela, il faudrait distinguer « jamais posé » de
/// « posé à zéro », alors que les deux veulent dire la même chose.
#[test]
fn la_generation_s_ecrit_meme_a_zero() {
    let ecrit = serde_json::to_string(&lock(Vec::new())).expect("sérialisation");
    assert!(ecrit.contains("\"generation\":0"), "{ecrit}");
}

/// L'empreinte porte sur ce que le verrou DIT, pas sur la façon dont il est
/// écrit.
///
/// C'est la propriété qui empêche de retélécharger huit cents mégaoctets
/// parce que l'hôte a changé son indentation : le même verrou minifié et le
/// même verrou indenté donnent la même empreinte.
#[test]
fn l_empreinte_ignore_la_mise_en_forme() {
    let indente = r#"{
        "schema": 1,
        "name": "samflix",
        "generated": "2026-09-17T00:00:00Z",
        "minecraft": "1.21.1",
        "loader": { "type": "neoforge", "version": "21.1.250" },
        "java": 21,
        "mods": []
    }"#;
    let minifie = r#"{"schema":1,"name":"samflix","generated":"2026-09-17T00:00:00Z","minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,"mods":[]}"#;

    let a = Lockfile::parse(indente.as_bytes())
        .unwrap()
        .empreinte()
        .unwrap();
    let b = Lockfile::parse(minifie.as_bytes())
        .unwrap()
        .empreinte()
        .unwrap();
    assert_eq!(a, b);
}

/// Mais elle ne rate pas un vrai changement : une version de mod qui bouge
/// doit donner une empreinte différente, sans quoi le launcher ne verrait
/// jamais une mise à jour.
#[test]
fn l_empreinte_voit_un_vrai_changement() {
    let avant = lock(vec![locked("jei", "abc", "19.51.0")]);
    let apres = lock(vec![locked("jei", "def", "19.52.0")]);
    assert_ne!(avant.empreinte().unwrap(), apres.empreinte().unwrap());
}

/// Et un changement de génération, à mods identiques, change l'empreinte lui
/// aussi : c'est ce qui fait qu'un pack à purger se distingue d'un pack
/// inchangé.
#[test]
fn l_empreinte_voit_un_changement_de_generation() {
    let avant = lock(Vec::new());
    let mut apres = lock(Vec::new());
    apres.generation = 1;
    assert_ne!(avant.empreinte().unwrap(), apres.empreinte().unwrap());
}

/// L'empreinte d'un verrou écrit puis relu est celle du verrou d'origine.
/// C'est la condition pour que `save` et `empreinte` restent cohérents : elles
/// passent par la même sérialisation, et non par deux qui se ressemblent.
#[test]
fn ecrire_puis_relire_conserve_l_empreinte() {
    let atelier = Atelier::neuf("verrou-empreinte");
    let chemin = atelier.racine.join("samflix.lock.json");

    let ecrit = verrou(vec![entree("jei", "both", Some(b"jei"))]);
    ecrit.save(&chemin).expect("écriture");

    let relu = Lockfile::load(&chemin).expect("lecture");
    assert_eq!(ecrit.empreinte().unwrap(), relu.empreinte().unwrap());
}
