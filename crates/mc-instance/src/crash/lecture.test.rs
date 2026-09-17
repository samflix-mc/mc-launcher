use super::{EXCERPT_LINES, parse};

/// L'extrait prend cinq lignes avant l'exception — ce que le jeu était en
/// train de faire — et soixante après. Un fichier de plantage en fait des
/// milliers : tout joindre ferait refuser l'événement, n'en joindre aucune
/// laisserait un incident sans trace.
#[test]
fn l_extrait_cadre_l_exception_sans_emporter_tout_le_fichier() {
    let mut lignes: Vec<String> = (0..20).map(|i| format!("avant-{i:02}")).collect();
    lignes.push("java.lang.NullPointerException: rien".to_string());
    lignes.extend((0..200).map(|i| format!("\tat quelque.part(Chose.java:{i})")));

    let crash = parse(&lignes.join("\n")).expect("exception trouvée");
    let extrait: Vec<&str> = crash.excerpt.lines().collect();

    // Cinq lignes de contexte, puis l'exception et ce qui la suit.
    assert_eq!(extrait.len(), 5 + EXCERPT_LINES);
    assert_eq!(extrait[0], "avant-15");
    assert_eq!(extrait[5], "java.lang.NullPointerException: rien");
    // La soixantième ligne à partir de l'exception, et pas une de plus.
    assert!(
        extrait.last().unwrap().contains("Chose.java:58"),
        "dernière ligne : {:?}",
        extrait.last()
    );
}

#[test]
fn reconnait_une_exception_de_resolution_de_modules() {
    // Le cas réel qui a empêché le premier lancement d'aboutir.
    let texte = "\
[04:01:48] [main/ERROR] [cpw.mods.modlauncher.Launcher/MODLAUNCHER]: Exception
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package com.mojang.blaze3d.systems to module bookshelf
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source) ~[?:?]
\tat java.base/java.lang.module.Resolver.failTwoSuppliers(Unknown Source) ~[?:?]";

    let crash = parse(texte).expect("exception trouvée");
    assert_eq!(crash.exception, "java.lang.module.ResolutionException");
    assert!(crash.message.starts_with("Modules _1._21._1 and minecraft"));
    assert!(crash.excerpt.contains("resolveFail"));
}

#[test]
fn la_premiere_exception_l_emporte() {
    // Les suivantes découlent souvent de la première.
    let texte = "\
java.lang.NullPointerException: rien
\tat quelque.part(Chose.java:1)
Caused by: java.lang.IllegalStateException: conséquence";
    assert_eq!(
        parse(texte).unwrap().exception,
        "java.lang.NullPointerException"
    );
}

#[test]
fn un_caused_by_seul_est_reconnu() {
    let texte = "Caused by: java.io.IOException: disque plein";
    let crash = parse(texte).unwrap();
    assert_eq!(crash.exception, "java.io.IOException");
    assert_eq!(crash.message, "disque plein");
}

#[test]
fn un_journal_ordinaire_ne_produit_pas_de_crash() {
    // Sans ce filtre, chaque lancement réussi remonterait un faux incident.
    let texte = "\
[04:01:47] [main/INFO] [Launcher/MODLAUNCHER]: ModLauncher running: args [--username, Sam]
[04:01:48] [main/INFO] [ModDiscoverer/SCAN]: Found mod file \"jei-1.21.1.jar\"
[04:01:50] [Render thread/INFO] [minecraft/Minecraft]: Setting user: Sam";
    assert!(parse(texte).is_none());
}
