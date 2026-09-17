use super::classpath;
use crate::essais::{Arbre, NEOFORGE, VANILLA};
use crate::launch::descripteur::resolve_chain;

const OS: &str = "linux";
const ARCH: &str = "x86_64";

/// NeoForge remplace certaines bibliothèques de Mojang. Sa version doit passer
/// devant, sinon la JVM charge celle de Mojang et le chargeur échoue sur une
/// méthode absente.
#[test]
fn la_bibliotheque_du_chargeur_remplace_celle_du_jeu() {
    let arbre = Arbre::neuf("cp-remplacement");
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .bibliotheque("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .bibliotheque("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");

    let chaine = resolve_chain(&arbre.shared(), "neoforge-21.1.250").unwrap();
    let (chemins, texte) = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").unwrap();

    let noms: Vec<String> = chemins
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(
        noms.contains(&"guava-33.0.0-jre.jar".to_string()),
        "{noms:?}"
    );
    assert!(
        !noms.contains(&"guava-32.1.2-jre.jar".to_string()),
        "les deux guava sont au classpath : {noms:?}"
    );
    assert_eq!(texte.split(':').count(), chemins.len());
}

/// Sous un chargeur, l'installateur a produit sa propre découpe du client.
/// Ajouter `1.21.1.jar` par-dessus donne deux modules qui exportent les mêmes
/// paquets, et la JVM s'arrête avant le premier écran.
#[test]
fn le_client_vanilla_reste_hors_du_classpath_sous_un_chargeur() {
    let arbre = Arbre::neuf("cp-client-charge");
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .bibliotheque("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .bibliotheque("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");

    let chaine = resolve_chain(&arbre.shared(), "neoforge-21.1.250").unwrap();
    let (chemins, _) = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").unwrap();

    assert!(
        !chemins.iter().any(|p| p.ends_with("1.21.1.jar")),
        "le client vanilla est au classpath : {chemins:?}"
    );
}

/// En vanilla pur, en revanche, c'est lui qui porte le jeu.
#[test]
fn le_client_vanilla_rejoint_le_classpath_sans_chargeur() {
    let arbre = Arbre::neuf("cp-client-nu");
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .bibliotheque("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let (chemins, _) = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").unwrap();

    assert!(
        chemins.iter().any(|p| p.ends_with("1.21.1.jar")),
        "{chemins:?}"
    );
}

/// Une bibliothèque réservée à un autre système n'a pas à être téléchargée ni
/// exigée : les deux tiers du classpath vanilla sont sans règle, le reste
/// dépend du système.
#[test]
fn une_bibliotheque_d_un_autre_systeme_est_ignoree() {
    let arbre = Arbre::neuf("cp-autre-systeme");
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .bibliotheque("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let (chemins, _) = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").unwrap();

    assert!(
        !chemins
            .iter()
            .any(|p| p.to_string_lossy().contains("lwjgl")),
        "une bibliothèque macOS a été retenue sous linux : {chemins:?}"
    );
}

/// Une bibliothèque manquante fait échouer le démarrage de toute façon ; le
/// dire ici, avec le nom du premier fichier absent, épargne une trace Java.
#[test]
fn une_bibliotheque_manquante_est_nommee() {
    let arbre = Arbre::neuf("cp-manquante");
    arbre.version("1.21.1", VANILLA).client("1.21.1");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let erreur = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").expect_err("guava est absent");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("guava"), "{texte}");
    assert!(texte.contains("relancer l'installation"), "{texte}");
}

#[test]
fn un_client_absent_est_dit_tel_quel() {
    let arbre = Arbre::neuf("cp-sans-client");
    arbre
        .version("1.21.1", VANILLA)
        .bibliotheque("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let erreur = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").expect_err("aucun client");

    assert!(
        format!("{erreur:#}").contains("client absent"),
        "{erreur:#}"
    );
}

/// Une bibliothèque sans `downloads` retombe sur le chemin Maven déduit de son
/// nom — systématique pour celles qu'ajoute NeoForge.
#[test]
fn une_bibliotheque_sans_telechargement_passe_par_son_chemin_maven() {
    let arbre = Arbre::neuf("cp-maven");
    arbre
        .version(
            "1.21.1",
            r#"{"id":"1.21.1","mainClass":"M","assetIndex":{"id":"17"},
                "libraries":[{"name":"net.neoforged:mergetool:2.0.0"}]}"#,
        )
        .client("1.21.1")
        .bibliotheque("net/neoforged/mergetool/2.0.0/mergetool-2.0.0.jar");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let (chemins, _) = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").unwrap();

    assert!(
        chemins.iter().any(|p| p.ends_with("mergetool-2.0.0.jar")),
        "{chemins:?}"
    );
}

/// Un nom qui ne ressemble à rien ne donne aucun chemin : mieux vaut s'arrêter
/// que composer un classpath silencieusement incomplet.
#[test]
fn un_nom_de_bibliotheque_inexploitable_arrete_l_assemblage() {
    let arbre = Arbre::neuf("cp-nom-casse");
    arbre
        .version(
            "1.21.1",
            r#"{"id":"1.21.1","mainClass":"M","assetIndex":{"id":"17"},
                "libraries":[{"name":"sansdeuxpoints"}]}"#,
        )
        .client("1.21.1");

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    let erreur = classpath(&chaine, &arbre.shared(), OS, ARCH, ":").expect_err("nom inexploitable");

    assert!(
        format!("{erreur:#}").contains("sans chemin exploitable"),
        "{erreur:#}"
    );
}
