use super::{Watcher, loaded_mods};

/// Donne les lignes à un observateur neuf, et rend ce qu'il en a tiré.
fn observer(lignes: &[&str]) -> Vec<super::Crash> {
    let mut watcher = Watcher::new();
    for ligne in lignes {
        watcher.line(ligne);
    }
    watcher.finish()
}

#[test]
fn une_partie_sans_exception_ne_remonte_rien() {
    let trouve = observer(&[
        "[04:01:48] [main/INFO]: Setting user: Sam",
        "[04:01:49] [Render thread/INFO]: OpenAL initialized.",
    ]);
    assert!(trouve.is_empty(), "{trouve:?}");
}

/// Une trace se poursuit par ses cadres ; tant qu'ils arrivent, ils
/// appartiennent à l'exception en cours et non à une nouvelle.
#[test]
fn les_cadres_de_pile_rejoignent_l_exception_qu_ils_decrivent() {
    let trouve = observer(&[
        "java.lang.NullPointerException: rien du tout",
        "\tat net.minecraft.Foo(Foo.java:1)",
        "\tat net.minecraft.Bar(Bar.java:2)",
        "Caused by: java.lang.IllegalStateException: conséquence",
        "\t... 12 more",
        "[04:02:00] [main/INFO]: la partie continue",
    ]);

    assert_eq!(trouve.len(), 1, "{trouve:?}");
    assert_eq!(trouve[0].exception, "java.lang.NullPointerException");
    assert!(trouve[0].excerpt.contains("Foo.java:1"), "{:?}", trouve[0]);
    assert!(trouve[0].excerpt.contains("... 12 more"), "{:?}", trouve[0]);
}

/// Un mod qui échoue à chaque tick remplirait le tableau de bord à lui seul :
/// la même exception ne compte qu'une fois.
#[test]
fn la_meme_exception_repetee_ne_compte_qu_une_fois() {
    let mut lignes = Vec::new();
    for _ in 0..10 {
        lignes.push("java.lang.NullPointerException: toujours la même");
        lignes.push("\tat net.minecraft.Tick(Tick.java:1)");
    }
    let refs: Vec<&str> = lignes.clone();

    let trouve = observer(&refs);
    assert_eq!(trouve.len(), 1, "{trouve:?}");
}

/// Au-delà de cinq exceptions distinctes, on cesse de remonter : une session
/// qui en produit tant a un problème global, que les premières décrivent déjà.
#[test]
fn au_dela_de_cinq_exceptions_distinctes_on_cesse_de_compter() {
    let lignes: Vec<String> = (0..12)
        .map(|n| format!("java.lang.IllegalStateException: cas numéro {n}"))
        .collect();
    let refs: Vec<&str> = lignes.iter().map(String::as_str).collect();

    let trouve = observer(&refs);
    assert_eq!(trouve.len(), 5, "{trouve:?}");
}

/// La sortie du jeu porte des codes de couleur ANSI : les laisser dans
/// l'extrait rendrait l'incident illisible dans le tableau de bord.
#[test]
fn les_couleurs_du_terminal_sont_retirees() {
    let trouve = observer(&["\u{1b}[31mjava.io.IOException: disque plein\u{1b}[0m"]);

    assert_eq!(trouve.len(), 1, "{trouve:?}");
    assert_eq!(trouve[0].exception, "java.io.IOException");
    assert!(!trouve[0].excerpt.contains('\u{1b}'), "{:?}", trouve[0]);
}

/// L'exception en cours au moment où la partie s'arrête ne doit pas être
/// perdue : c'est souvent celle qui a tout arrêté.
#[test]
fn l_exception_en_cours_a_la_fin_est_conservee() {
    let trouve = observer(&[
        "java.lang.OutOfMemoryError: Java heap space",
        "\tat net.minecraft.Chunk(Chunk.java:1)",
    ]);

    assert_eq!(trouve.len(), 1, "{trouve:?}");
    assert_eq!(trouve[0].exception, "java.lang.OutOfMemoryError");
    assert_eq!(
        trouve[0].source,
        std::path::PathBuf::from("sortie du jeu"),
        "la provenance doit dire que rien n'a été lu sur le disque"
    );
}

#[test]
fn deux_exceptions_differentes_sont_toutes_deux_retenues() {
    let trouve = observer(&[
        "java.io.IOException: disque plein",
        "[INFO]: on continue",
        "java.lang.NullPointerException: autre chose",
    ]);

    assert_eq!(trouve.len(), 2, "{trouve:?}");
}

/// Un plantage de modpack vient presque toujours d'un mod ou d'un couple de
/// mods ; savoir lesquels étaient présents épargne un aller-retour.
#[test]
fn les_mods_charges_sont_listes_dans_l_ordre() {
    let arbre = crate::essais::Arbre::neuf("mods-charges");
    let mods = arbre.game_dir().join("mods");
    std::fs::create_dir_all(&mods).unwrap();
    std::fs::write(mods.join("sodium.jar"), b"").unwrap();
    std::fs::write(mods.join("jei.jar"), b"").unwrap();
    // Ni les fichiers désactivés, ni la configuration laissée là.
    std::fs::write(mods.join("bug.jar.disabled"), b"").unwrap();

    let noms = loaded_mods(&arbre.game_dir()).unwrap();
    assert_eq!(noms, vec!["jei.jar", "sodium.jar"]);
}

/// Un répertoire de mods absent n'est pas une faute : c'est le cas d'une
/// instance vanilla, et le rapport de plantage doit partir quand même.
#[test]
fn un_repertoire_de_mods_absent_donne_une_liste_vide() {
    let arbre = crate::essais::Arbre::neuf("mods-absents");
    assert!(loaded_mods(&arbre.game_dir()).unwrap().is_empty());
}

/// Une trace peut faire des milliers de cadres — une récursion infinie en
/// produit jusqu'à ce que la pile cède. L'extrait s'arrête à soixante lignes :
/// ce qui suit n'apprend rien, et un événement démesuré se fait refuser.
#[test]
fn une_trace_interminable_est_bornee() {
    let mut lignes = vec!["java.lang.StackOverflowError: pile pleine".to_string()];
    lignes.extend((0..200).map(|i| format!("\tat net.minecraft.Recursion(R.java:{i})")));
    let vues: Vec<&str> = lignes.iter().map(String::as_str).collect();

    let trouve = observer(&vues);
    assert_eq!(trouve.len(), 1, "{trouve:?}");

    // La déclaration, plus soixante cadres : pas cinquante-neuf, pas soixante
    // et un. C'est la borne elle-même qu'on vérifie, et elle ne tient qu'à un
    // compteur qui avance d'un à chaque ligne retenue.
    assert_eq!(
        trouve[0].excerpt.lines().count(),
        1 + super::EXCERPT_LINES,
        "{:?}",
        trouve[0]
    );
    assert!(trouve[0].excerpt.contains("R.java:59"), "{:?}", trouve[0]);
    assert!(!trouve[0].excerpt.contains("R.java:60"), "{:?}", trouve[0]);
}
