use super::{Partie, lignes};
use mc_instance::launch::{Command, Session};

/// Une partie prête à démarrer, telle que `preparer` la rend.
fn partie(cible: Option<&str>, demande_explicite: bool) -> Partie {
    let layout = mc_instance::Layout::new(std::env::temp_dir().join("mc-pack-annonce"));
    Partie {
        instance: layout.instance("samflix"),
        lock: crate::commandes::essais::verrou(Vec::new()),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/usr/bin/java"),
            args: Vec::new(),
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "b50ad385829d3141a2167e7d7539ba7f"),
        cible: cible.map(str::to_string),
        demande_explicite,
        environnement: mc_log::Environment::Production,
    }
}

/// C'est la dernière chose qu'un joueur lit avant que le jeu ne prenne la
/// main, et la première qu'il recopie quand il demande de l'aide. Chaque ligne
/// répond à une question posée en vrai.
#[test]
fn le_recapitulatif_nomme_l_instance_la_version_le_joueur_et_les_mods() {
    let partie = partie(None, false);
    let rendu = lignes(&partie).join("\n");

    assert!(rendu.contains("samflix"), "{rendu}");
    assert!(rendu.contains("neoforge-21.1.250"), "{rendu}");
    assert!(rendu.contains("Sam"), "{rendu}");
    assert!(
        rendu.contains("b50ad385829d3141a2167e7d7539ba7f"),
        "l'UUID manque : {rendu}"
    );
    assert!(rendu.contains("mods"), "{rendu}");
}

/// D'où vient l'adresse compte autant qu'elle : « mc.exemple.fr (production) »
/// laissait croire que l'hôte venait du pack, alors qu'un `--serveur` peut
/// désigner n'importe quoi. Quelqu'un qui diagnostique une éjection a besoin de
/// savoir lequel des deux il regarde.
#[test]
fn le_serveur_dit_d_ou_vient_son_adresse() {
    let demande = lignes(&partie(Some("mc.exemple.fr"), true)).join("\n");
    assert!(demande.contains("mc.exemple.fr"), "{demande}");
    assert!(demande.contains("ligne de commande"), "{demande}");

    let du_pack = lignes(&partie(Some("mc.exemple.fr"), false)).join("\n");
    assert!(du_pack.contains("déclaré par le pack"), "{du_pack}");
    // La clé affichée est celle réellement lue dans le manifeste.
    assert!(du_pack.contains("production"), "{du_pack}");
}

/// La préproduction n'a pas de serveur derrière elle : le jeu s'y ouvre sur le
/// menu, et le dire évite de chercher une panne de connexion.
#[test]
fn sans_serveur_le_menu_est_annonce() {
    let rendu = lignes(&partie(None, false)).join("\n");
    assert!(rendu.contains("aucun"), "{rendu}");
    assert!(rendu.contains("menu"), "{rendu}");
}
