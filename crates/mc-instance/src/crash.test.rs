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

#[test]
fn un_horodatage_n_est_pas_pris_pour_une_exception() {
    // « 04:01:47 » contient des deux-points et des chiffres.
    assert!(split_exception("[04:01:47] [main/INFO] démarrage").is_none());
}

#[test]
fn un_chemin_de_fichier_n_est_pas_pris_pour_une_exception() {
    assert!(split_exception("fichier: /home/sam/.local/share/truc.jar").is_none());
}

#[test]
fn une_ligne_de_trace_n_est_pas_une_declaration() {
    assert!(split_exception("\tat java.base/java.lang.Thread.run(Thread.java:1)").is_none());
}

#[test]
fn les_couleurs_sont_retirees() {
    let colore = "\u{1b}[32m[04:01] ERREUR\u{1b}[m suite";
    assert_eq!(strip_ansi(colore), "[04:01] ERREUR suite");
}

#[test]
fn une_exception_coloree_reste_reconnaissable() {
    // Minecraft colore sa sortie ; sans le nettoyage, rien ne correspond.
    let ligne = "\u{1b}[31mjava.lang.OutOfMemoryError: Java heap space\u{1b}[m";
    let (exception, message) = split_exception(ligne).unwrap();
    assert_eq!(exception, "java.lang.OutOfMemoryError");
    assert_eq!(message, "Java heap space");
}

#[test]
fn une_classe_qui_n_est_pas_une_exception_est_ecartee() {
    // « net.minecraft.client.Minecraft: démarrage » n'est pas un plantage.
    assert!(split_exception("net.minecraft.client.Minecraft: démarrage").is_none());
}
