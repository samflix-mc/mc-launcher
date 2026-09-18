use super::super::descripteur::AssetObject;
use super::{compter_les_telechargements, poids};
use mc_dl::Fetched;

/// Le compte sépare une première installation — des milliers d'objets — d'une
/// seconde, où tout est déjà là. C'est la seule explication qu'on ait de
/// l'attente : annoncer zéro quand on vient de descendre trois mille fichiers
/// ferait passer une demi-heure pour une anomalie.
#[test]
fn seuls_les_objets_reellement_descendus_sont_comptes() {
    let resultats = vec![
        Ok(Fetched::Downloaded),
        Ok(Fetched::AlreadyPresent),
        Ok(Fetched::Downloaded),
        Ok(Fetched::AlreadyPresent),
    ];

    assert_eq!(compter_les_telechargements(resultats).unwrap(), 2);

    // Une seconde installation ne descend rien, et le dit.
    let tout_present = vec![Ok(Fetched::AlreadyPresent), Ok(Fetched::AlreadyPresent)];
    assert_eq!(compter_les_telechargements(tout_present).unwrap(), 0);

    assert_eq!(compter_les_telechargements(Vec::new()).unwrap(), 0);
}

/// La première erreur arrête tout : un asset manquant fait une texture
/// absente, pas un jeu qui refuse de démarrer, et l'on préfère le savoir tout
/// de suite plutôt qu'au premier écran noir.
#[test]
fn un_objet_en_echec_arrete_le_compte() {
    let resultats = vec![
        Ok(Fetched::Downloaded),
        Err(anyhow::anyhow!("asset abc123 : 404")),
        Ok(Fetched::Downloaded),
    ];

    let erreur = compter_les_telechargements(resultats).expect_err("l'échec remonte");
    assert!(format!("{erreur:#}").contains("abc123"), "{erreur:#}");
}

/// Le poids du lot est la SOMME des tailles annoncées.
///
/// Le commentaire de la fonction dit qu'elle est « séparée de la boucle pour
/// être vérifiable » — elle ne l'était pas : rendre zéro, ou un, laissait la
/// suite verte. Or c'est ce total qui fixe l'échelle de la barre de
/// progression. Rendre zéro la mettrait à cent pour cent dès le premier
/// octet ; rendre un la ferait déborder de trois mille fois.
#[test]
fn le_poids_du_lot_est_la_somme_des_tailles() {
    let objets = vec![
        AssetObject {
            hash: "a".into(),
            size: 1_024,
        },
        AssetObject {
            hash: "b".into(),
            size: 2_048,
        },
        AssetObject {
            hash: "c".into(),
            size: 1,
        },
    ];

    assert_eq!(poids(&objets), 3_073);
    // Un lot vide pèse zéro — et c'est le seul cas où zéro est juste.
    assert_eq!(poids(&[]), 0);
}

/// Et la somme ne déborde pas sur un index réel.
///
/// Trois mille objets d'assets, c'est l'ordre de grandeur de Minecraft 1.21 ;
/// le total dépasse largement ce qu'un `u32` tiendrait.
#[test]
fn le_poids_tient_un_index_entier() {
    let objets: Vec<AssetObject> = (0..3_000)
        .map(|i| AssetObject {
            hash: format!("{i:040x}"),
            size: 5_000_000,
        })
        .collect();

    assert_eq!(poids(&objets), 15_000_000_000);
}
