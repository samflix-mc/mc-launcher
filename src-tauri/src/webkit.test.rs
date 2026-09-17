//! La décision se vérifie ; le pilote de la machine qui exécute les tests, non.
//!
//! `regler_le_rendu` lit `/sys/module/nvidia` et écrit l'environnement du
//! processus : son résultat dépend du poste, et son effet fuiterait d'un test
//! à l'autre. C'est `doit_desactiver_dmabuf` qui porte le choix, et elle prend
//! ses deux entrées en argument exactement pour ça.

use super::*;

#[test]
fn nvidia_seul_declenche_le_contournement() {
    assert!(doit_desactiver_dmabuf(true, false));
}

#[test]
fn sans_nvidia_le_rendu_reste_intact() {
    // DMA-BUF fonctionne ailleurs, et le couper coûterait une copie par image.
    assert!(!doit_desactiver_dmabuf(false, false));
}

#[test]
fn un_choix_explicite_n_est_jamais_ecrase() {
    // Y compris sur NVIDIA : poser la variable à « 0 » est le seul moyen de
    // réclamer DMA-BUF quand le pilote a été corrigé.
    assert!(!doit_desactiver_dmabuf(true, true));
    assert!(!doit_desactiver_dmabuf(false, true));
}

#[test]
fn les_deux_noms_ne_bougent_pas() {
    // L'un est lu par WebKitGTK, l'autre par le noyau : une faute de frappe ne
    // casse rien à la compilation et rend le contournement muet.
    assert_eq!(VARIABLE, "WEBKIT_DISABLE_DMABUF_RENDERER");
    assert_eq!(MODULE_NVIDIA, "/sys/module/nvidia");
}
