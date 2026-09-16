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
