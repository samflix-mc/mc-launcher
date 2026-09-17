use super::choisir;
use crate::essais::environnement;
use crate::manifest::Manifest;

const SERVEURS: &str = r#"{"production":{"host":"mc.ggy.info"},
                           "development":{"host":"78.46.100.5","port":25566}}"#;

fn manifeste(serveurs: &str) -> Manifest {
    let brut = format!(
        r#"{{"schema":1,"name":"samflix","minecraft":"1.21.1",
             "loader":{{"type":"neoforge","version":"21.1.250"}},
             "servers":{serveurs}}}"#
    );
    Manifest::parse(brut.as_bytes()).unwrap()
}

/// `--serveur` prime sur ce que le pack déclare, et la provenance est retenue :
/// quelqu'un qui diagnostique une éjection a besoin de savoir lequel des deux
/// il regarde.
#[test]
fn une_demande_explicite_prime_et_se_sait() {
    let _env = environnement("production");

    let (cible, explicite, _) =
        choisir(&manifeste(SERVEURS), Some("essai.exemple.fr:25577".into()));

    assert_eq!(cible.as_deref(), Some("essai.exemple.fr:25577"));
    assert!(explicite);
}

/// À défaut, celui que le pack déclare pour l'environnement de ce binaire. Le
/// manifeste est le même partout : c'est au client de choisir, et il choisit
/// avec ce que la CI lui a figé à la compilation.
#[test]
fn a_defaut_le_pack_decide_selon_l_environnement() {
    let env = environnement("production");

    let (cible, explicite, environnement) = choisir(&manifeste(SERVEURS), None);
    // Sans port déclaré, l'adresse s'en tient à l'hôte : le jeu prend 25565.
    assert_eq!(cible.as_deref(), Some("mc.ggy.info"));
    assert!(!explicite);
    assert_eq!(environnement, mc_log::Environment::Production);

    env.poser("development");
    let (cible, _, _) = choisir(&manifeste(SERVEURS), None);
    assert_eq!(cible.as_deref(), Some("78.46.100.5:25566"));
}

/// Une absence n'est pas une erreur : la préproduction n'a pas de serveurs
/// Minecraft derrière elle, et le jeu s'y lance sans rejoindre quoi que ce soit.
#[test]
fn aucun_serveur_declare_n_est_pas_une_erreur() {
    let _env = environnement("preproduction");

    let (cible, explicite, _) = choisir(&manifeste(SERVEURS), None);
    assert!(cible.is_none(), "{cible:?}");
    assert!(!explicite);
}
