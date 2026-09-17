use super::vaut_d_etre_annonce;

/// Le dernier tour d'une résolution ne descend rien : il relit ce que les
/// précédents ont posé. Annoncer ces tours-là noierait la ligne qui compte ;
/// ne plus rien annoncer priverait le joueur du seul signe que l'installation
/// avance.
#[test]
fn seul_un_tour_qui_telecharge_s_annonce() {
    assert!(!vaut_d_etre_annonce(0));
    assert!(vaut_d_etre_annonce(1));
    assert!(vaut_d_etre_annonce(120));
}
