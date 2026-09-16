use super::*;

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
