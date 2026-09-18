use super::{Billet, date_valide, ordonner};

fn billet(id: &str, date: &str, epinglee: bool) -> Billet {
    Billet {
        id: id.into(),
        titre: id.into(),
        date: date.into(),
        epinglee,
        image: None,
        corps: Vec::new(),
    }
}

fn ids(billets: &[Billet]) -> Vec<&str> {
    billets.iter().map(|b| b.id.as_str()).collect()
}

/// Les épinglés d'abord, puis du plus récent au plus ancien.
///
/// Le tri est en Rust et non dans un `computed()` d'Angular, pour la même
/// raison que la règle du bouton : une règle qui vit côté front ne se vérifie
/// qu'en vitest et sort du périmètre de mutation. Celle-ci est exactement le
/// genre dont l'inversion ne casse aucun test d'affichage et ne se voit qu'à
/// l'œil, des semaines plus tard.
#[test]
fn les_epingles_d_abord_puis_du_plus_recent_au_plus_ancien() {
    let trie = ordonner(vec![
        billet("vieux", "2026-01-01T00:00:00Z", false),
        billet("epingle-vieux", "2025-01-01T00:00:00Z", true),
        billet("recent", "2026-09-01T00:00:00Z", false),
        billet("epingle-recent", "2026-08-01T00:00:00Z", true),
    ]);

    assert_eq!(
        ids(&trie),
        vec!["epingle-recent", "epingle-vieux", "recent", "vieux"]
    );
}

/// À date et épinglage égaux, l'identifiant tranche.
///
/// Sans ce dernier critère, l'ordre dépendrait de celui du JSON reçu, et deux
/// chargements du même fil pourraient ne pas donner le même écran — ce qui se
/// remarque à peine, et se diagnostique très mal.
#[test]
fn a_egalite_l_identifiant_tranche() {
    let un = ordonner(vec![
        billet("b", "2026-09-01T00:00:00Z", false),
        billet("a", "2026-09-01T00:00:00Z", false),
    ]);
    let deux = ordonner(vec![
        billet("a", "2026-09-01T00:00:00Z", false),
        billet("b", "2026-09-01T00:00:00Z", false),
    ]);
    assert_eq!(ids(&un), vec!["a", "b"]);
    assert_eq!(ids(&un), ids(&deux));
}

#[test]
fn un_fil_vide_se_trie_sans_paniquer() {
    assert!(ordonner(Vec::new()).is_empty());
}

// --- La date ---------------------------------------------------------------

#[test]
fn une_date_rfc3339_en_utc_est_valide() {
    assert!(date_valide("2026-09-18T18:00:00Z"));
    assert!(date_valide("1999-12-31T23:59:59Z"));
}

/// Un décalage horaire est REFUSÉ, et ce n'est pas de la rigidité : le tri
/// compare les chaînes telles quelles, ce qui n'est l'ordre chronologique que
/// si toutes sont en UTC avec le même nombre de chiffres. Une date en
/// « +02:00 » casserait cette propriété, et rien dans l'affichage ne le
/// montrerait — les billets seraient simplement dans un ordre un peu faux.
#[test]
fn un_decalage_horaire_est_refuse() {
    assert!(!date_valide("2026-09-18T18:00:00+02:00"));
    assert!(!date_valide("2026-09-18T18:00:00-05:00"));
}

#[test]
fn les_dates_malformees_sont_refusees() {
    for mauvaise in [
        "",
        "2026-09-18",
        "2026-09-18 18:00:00Z",   // espace au lieu de T
        "2026/09/18T18:00:00Z",   // séparateurs
        "26-09-18T18:00:00Z",     // année sur deux chiffres
        "2026-09-18T18:00:00",    // pas de Z
        "2026-09-18T18:00:00.5Z", // fraction de seconde
        "aaaa-bb-ccTdd:ee:ffZ",
    ] {
        assert!(!date_valide(mauvaise), "acceptée à tort : « {mauvaise} »");
    }
}
