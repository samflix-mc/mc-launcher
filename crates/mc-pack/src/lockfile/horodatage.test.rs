use super::format_iso8601;

#[test]
fn horodatage_iso8601() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    // Année bissextile, 29 février.
    assert_eq!(format_iso8601(1_709_164_800), "2024-02-29T00:00:00Z");
}

/// Le calcul du calendrier compte en « ères » de quatre cents ans, et corrige
/// les siècles non bissextiles par deux divisions dont on ne voit rien avant
/// 2100 : d'ici là, le terme vaut zéro et n'importe quel signe donne la même
/// date. Ces bornes-là sont les seules qui les mettent en jeu.
#[test]
fn les_siecles_non_bissextiles_sont_comptes() {
    // 1900, 2100, 2200 et 2300 ne sont pas bissextiles, 2000 et 2400 le sont.
    assert_eq!(format_iso8601(4_107_456_000), "2100-02-28T00:00:00Z");
    assert_eq!(format_iso8601(4_107_542_400), "2100-03-01T00:00:00Z");
    // Début de l'ère courante, et son tout dernier jour quatre cents ans plus
    // tard : les deux extrémités du cycle.
    assert_eq!(format_iso8601(951_868_800), "2000-03-01T00:00:00Z");
    assert_eq!(format_iso8601(13_574_563_200), "2400-02-29T00:00:00Z");
}
