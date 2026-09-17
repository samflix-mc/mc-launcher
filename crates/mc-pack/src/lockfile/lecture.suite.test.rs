use super::super::Lockfile;
use crate::essais::{Atelier, entree, verrou};

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
    assert_eq!(relu.pack, "samflix");
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

    let lock = Lockfile::from_plan(
        "samflix",
        "1.21.1",
        crate::lockfile::LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        21,
        &plan,
    );

    assert_eq!(lock.pack, "samflix");
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
