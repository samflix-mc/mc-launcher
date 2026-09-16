use super::*;
use crate::modrinth::api::ApiFile;

fn project(client: &str, server: &str) -> Project {
    Project {
        id: "id".into(),
        slug: "slug".into(),
        title: "Titre".into(),
        client_side: client.into(),
        server_side: server.into(),
    }
}

/// Modrinth publie les deux empreintes ; longtemps seule la plus faible
/// était lue, et c'est elle qui partait dans le verrou.
#[test]
fn le_sha512_publie_par_modrinth_est_retenu() {
    let brut = r#"{
        "url": "https://cdn.modrinth.com/jade.jar",
        "filename": "jade.jar",
        "primary": true,
        "size": 1024,
        "hashes": {
            "sha1": "0a385a583a1e9413ecf2a47d00000000deadbeef",
            "sha512": "b6c782de87e7259d997e199200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
        }
    }"#;
    let file: ApiFile = serde_json::from_str(brut).expect("fichier Modrinth lisible");
    assert_eq!(file.hashes.sha512.as_deref().map(str::len), Some(128));
    assert!(file.hashes.sha1.is_some());
}

#[test]
fn cote_deduit_des_metadonnees_du_projet() {
    assert_eq!(side_of(&project("required", "unsupported")), Side::Client);
    assert_eq!(side_of(&project("unsupported", "required")), Side::Server);
    assert_eq!(side_of(&project("required", "required")), Side::Both);
    // JEI et Jade sont « optional / optional » : installés des deux côtés,
    // faute de quoi les registres NeoForge divergeraient.
    assert_eq!(side_of(&project("optional", "optional")), Side::Both);
}
