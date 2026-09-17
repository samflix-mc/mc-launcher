use super::{split_exception, strip_ansi};

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

/// Une exception arrive presque toujours précédée de l'horodatage et de la
/// catégorie que pose le journal du jeu. Ce qui les sépare de la classe est un
/// « ]: » dont la longueur compte : d'un caractère trop tôt, la classe
/// emporterait la fin de la catégorie et ne serait plus reconnue.
#[test]
fn l_horodatage_et_la_categorie_sont_retires_avant_la_classe() {
    let ligne = "[04:01:47] [main/ERROR]: java.lang.NullPointerException: rien à cet endroit";
    let (exception, message) = split_exception(ligne).expect("l'exception suit la catégorie");
    assert_eq!(exception, "java.lang.NullPointerException");
    assert_eq!(message, "rien à cet endroit");
}

/// « Caused by: » introduit la cause réelle, et c'est elle qu'on veut : la
/// première exception d'une trace est souvent un emballage qui ne dit rien.
#[test]
fn la_cause_annoncee_est_celle_qui_est_retenue() {
    let (exception, message) =
        split_exception("Caused by: java.io.FileNotFoundException: mods/truc.jar")
            .expect("la cause est une exception");
    assert_eq!(exception, "java.io.FileNotFoundException");
    assert_eq!(message, "mods/truc.jar");
}

/// Trois conditions écartent ce qui ressemble à une classe sans en être une.
/// Chacune compte pour elle-même : les confondre laisserait passer une phrase
/// de journal pour un plantage, et le launcher remonterait un incident qui
/// n'existe pas.
#[test]
fn une_classe_sans_paquet_avec_espace_ou_avec_barre_n_en_est_pas_une() {
    // Sans point : une classe Java est nommée par un chemin pointé.
    assert!(split_exception("MonException: détail").is_none());
    // Avec un espace : c'est une phrase, pas un identifiant.
    assert!(split_exception("java.lang.Une Exception: détail").is_none());
    // Avec une barre : c'est un chemin de module ou de fichier.
    assert!(split_exception("java.base/java.lang.Exception: détail").is_none());
}
