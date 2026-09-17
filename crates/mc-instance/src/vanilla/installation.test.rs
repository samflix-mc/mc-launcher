use super::entree_de_version;
use crate::vanilla::descripteur::ManifestVersion;

fn version(id: &str) -> ManifestVersion {
    ManifestVersion {
        id: id.to_string(),
        url: format!("https://exemple.invalid/{id}.json"),
        sha1: "abc123".into(),
    }
}

/// Le manifeste de Mojang énumère plusieurs centaines de versions, instantanés
/// compris. Se tromper d'entrée installerait un autre jeu que celui demandé,
/// avec ses bibliothèques et ses assets — et le pack ne démarrerait pas, pour
/// une raison qui ne se lirait nulle part.
#[test]
fn seule_la_version_demandee_est_retenue() {
    let publiees = vec![version("1.21.4"), version("1.21.1"), version("25w07a")];

    let trouvee = entree_de_version(publiees, "1.21.1").expect("la version est publiée");
    assert_eq!(trouvee.id, "1.21.1");
    assert!(trouvee.url.contains("1.21.1"));
}

/// Une version que Mojang ne publie pas ne donne rien — c'est le cas d'une
/// faute de frappe dans le manifeste du pack, et l'appelant en fait un message
/// qui la nomme.
#[test]
fn une_version_absente_du_manifeste_ne_donne_rien() {
    assert!(entree_de_version(vec![version("1.21.1")], "1.21.9").is_none());
    assert!(entree_de_version(Vec::new(), "1.21.1").is_none());
}
