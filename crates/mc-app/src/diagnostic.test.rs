use super::{demande, rapport};

/// Le drapeau se reconnaît où qu'il soit : le lanceur d'un runner passe
/// parfois le chemin du binaire, parfois des arguments de session graphique
/// avant les nôtres.
#[test]
fn le_drapeau_se_reconnait_a_n_importe_quelle_place() {
    assert!(demande(["--diagnostic"]));
    assert!(demande(["helm", "--diagnostic"]));
    assert!(demande(["--diagnostic", "--autre"]));
}

/// Et rien d'autre ne le déclenche. Un joueur qui ouvre le launcher n'a aucun
/// argument ; un mot qui y ressemble ne doit pas escamoter sa fenêtre.
#[test]
fn rien_d_autre_ne_declenche_le_diagnostic() {
    assert!(!demande(Vec::<String>::new()));
    assert!(!demande(["helm"]));
    assert!(!demande(["--diagnostics"]));
    assert!(!demande(["diagnostic"]));
    assert!(!demande(["--diagnostic=1"]));
}

/// Le rapport porte les quatre lignes que la CI oppose à ce qu'elle a
/// construit. Les chercher par leur libellé et non par leur rang : une ligne
/// ajoutée en tête ne doit pas casser le contrôle.
#[test]
fn le_rapport_nomme_ce_que_la_ci_verifie() {
    let texte = rapport(false);

    for attendu in ["nom", "version", "environnement", "répertoire"] {
        assert!(
            texte.contains(attendu),
            "« {attendu} » absent du rapport :\n{texte}"
        );
    }
    // La version est celle du binaire, pas un littéral : c'est elle que le job
    // « coherence » de release.yml compare au tag.
    assert!(texte.contains(env!("CARGO_PKG_VERSION")), "{texte}");
}

/// L'état du rendu se lit dans les deux sens. C'est la première ligne à
/// demander à qui voit une fenêtre blanche sous NVIDIA, et une valeur figée
/// n'apprendrait rien.
#[test]
fn le_rapport_dit_ce_qui_a_ete_decide_du_rendu() {
    assert!(rapport(true).contains("désactivé"), "{}", rapport(true));
    assert!(
        rapport(false).contains("laissé à WebKit"),
        "{}",
        rapport(false)
    );
}
