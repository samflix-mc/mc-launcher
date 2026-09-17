use super::Command;

fn commande(args: &[&str]) -> Command {
    Command {
        java: std::path::PathBuf::from("/usr/lib/jvm/temurin-21/bin/java"),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        working_dir: std::path::PathBuf::from("/tmp/instance"),
    }
}

/// Le classpath fait plusieurs dizaines de milliers de caractères et personne
/// ne le lit : l'afficher noierait la ligne qu'on voulait relire.
#[test]
fn le_classpath_est_abrege_par_son_nombre_de_bibliotheques() {
    let affichage = commande(&["-cp", "/a.jar:/b.jar:/c.jar", "net.minecraft.Main"]).display();

    assert!(!affichage.contains("/a.jar"), "{affichage}");
    assert!(affichage.contains("<3 bibliothèques>"), "{affichage}");
    // Ce qui l'entoure reste lisible, et rejouable à la main.
    assert!(affichage.starts_with("/usr/lib/jvm/temurin-21/bin/java "));
    assert!(affichage.ends_with("net.minecraft.Main"));
}

/// Seul l'argument qui suit `-cp` est abrégé. Abréger le suivant aussi
/// masquerait la classe principale.
#[test]
fn seul_l_argument_du_classpath_est_abrege() {
    let affichage = commande(&["-Xmx4096M", "-cp", "/a.jar", "M", "--username", "Sam"]).display();

    assert!(affichage.contains("-Xmx4096M"), "{affichage}");
    assert!(affichage.contains("--username Sam"), "{affichage}");
    assert!(affichage.contains("<1 bibliothèques>"), "{affichage}");
}

#[test]
fn une_commande_sans_classpath_s_affiche_telle_quelle() {
    let affichage = commande(&["-version"]).display();
    assert!(affichage.ends_with("java -version"), "{affichage}");
}
