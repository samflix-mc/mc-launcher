use super::{demarree, en_cours, ordre_d_arret, terminee};

/// **Zéro veut dire « aucune partie », et rien d'autre.**
///
/// Le numéro laissé derrière une partie finie serait réutilisé par le système
/// pour un autre programme — et le bouton « arrêter » tuerait alors ce
/// programme-là. C'est la raison d'être de `terminee`.
#[test]
fn le_numero_se_pose_et_se_reprend() {
    terminee();
    assert_eq!(en_cours(), None);

    demarree(4242);
    assert_eq!(en_cours(), Some(4242));

    terminee();
    assert_eq!(en_cours(), None);
}

/// L'ordre d'arrêt, tel que le système l'attend.
///
/// `-KILL` et non `-TERM` : la JVM installe des gestionnaires pour le second,
/// et un processus figé — le seul cas où ce bouton sert — ne les exécute
/// jamais. C'est le genre de détail qu'une relecture laisse passer et qu'on ne
/// découvre qu'avec un jeu bloqué sous les yeux.
#[test]
fn l_ordre_d_arret_est_sans_menagement() {
    let (programme, arguments) = ordre_d_arret(1234);

    if cfg!(windows) {
        assert_eq!(programme, "taskkill");
        assert!(arguments.contains(&"/F".to_string()));
        assert!(arguments.contains(&"/T".to_string()));
    } else {
        assert_eq!(programme, "kill");
        assert_eq!(arguments, vec!["-KILL".to_string(), "1234".to_string()]);
    }
    assert!(arguments.contains(&"1234".to_string()));
}
