use super::compter_les_telechargements;
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
