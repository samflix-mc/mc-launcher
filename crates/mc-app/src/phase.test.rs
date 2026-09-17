use super::*;

#[test]
fn le_chemin_est_dans_l_ordre_ou_il_se_parcourt() {
    let rangs: Vec<usize> = Phase::TOUTES.iter().map(|phase| phase.rang()).collect();

    assert_eq!(rangs, (0..Phase::TOUTES.len()).collect::<Vec<_>>());
}

#[test]
fn les_etapes_de_mc_pack_gardent_leur_ordre_une_fois_traduites() {
    // Les sept étapes de l'installation doivent occuper sept rangs
    // consécutifs, et dans le même ordre : c'est ce qui permet à la fenêtre
    // d'avancer d'un cran à chaque étape reçue, sans jamais reculer.
    let rangs: Vec<usize> = mc_pack::Etape::TOUTES
        .iter()
        .map(|etape| Phase::from(*etape).rang())
        .collect();

    let premier = rangs[0];
    assert_eq!(
        rangs,
        (premier..premier + mc_pack::Etape::TOUTES.len()).collect::<Vec<_>>()
    );
}

#[test]
fn l_installation_est_encadree_par_ce_que_mc_pack_ignore() {
    // La connexion et la licence viennent avant — mc-pack n'authentifie rien —
    // et le lancement après. Si l'une d'elles passait du mauvais côté, le
    // chemin affiché reculerait en cours de route.
    assert!(Phase::Connexion.rang() < Phase::from(mc_pack::Etape::Pack).rang());
    assert!(Phase::Licence.rang() < Phase::from(mc_pack::Etape::Pack).rang());
    assert!(Phase::Pret.rang() > Phase::from(mc_pack::Etape::Verrou).rang());
    assert!(Phase::Lancement.rang() > Phase::Pret.rang());
}

#[test]
fn chaque_phase_se_serialise_sous_un_nom_distinct() {
    // Ces noms sont la clé côté TypeScript : deux phases qui partagent le même
    // en éclaireraient une seule.
    let noms: std::collections::BTreeSet<String> = Phase::TOUTES
        .iter()
        .map(|phase| serde_json::to_string(phase).expect("sérialisation"))
        .collect();

    assert_eq!(noms.len(), Phase::TOUTES.len());
}

#[test]
fn les_identifiants_serialises_ne_bougent_pas() {
    assert_eq!(
        serde_json::to_string(&Phase::NeoForge).expect("sérialisation"),
        "\"neo-forge\""
    );
    assert_eq!(
        serde_json::to_string(&Phase::Connexion).expect("sérialisation"),
        "\"connexion\""
    );
}

#[test]
fn chaque_phase_a_un_libelle_non_vide() {
    // Un libellé vide laisserait une ligne muette dans le chemin affiché.
    for phase in Phase::TOUTES {
        assert!(!phase.libelle().is_empty(), "{phase:?}");
    }
}
