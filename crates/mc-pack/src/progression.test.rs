use super::*;

#[test]
fn les_etapes_sont_dans_l_ordre_ou_elles_surviennent() {
    // L'ordre n'est pas cosmétique : il est celui des dépendances. Mojang
    // donne la version de Java exigée, NeoForge a besoin de ce Java, les mods
    // du chargeur posé. Une interface qui afficherait le chemin dans un autre
    // ordre mentirait sur ce qui reste à faire.
    let rangs: Vec<usize> = Etape::TOUTES.iter().map(|etape| etape.rang()).collect();

    assert_eq!(rangs, (0..Etape::TOUTES.len()).collect::<Vec<_>>());
}

#[test]
fn chaque_etape_a_un_identifiant_distinct() {
    // Ces chaînes traversent le pont vers l'interface et servent de clé. Deux
    // étapes qui partagent la même en éclaireraient une seule.
    let noms: std::collections::BTreeSet<&str> =
        Etape::TOUTES.iter().map(|etape| etape.as_str()).collect();

    assert_eq!(noms.len(), Etape::TOUTES.len());
}

#[test]
fn les_identifiants_ne_bougent_pas() {
    // Le TypeScript compare à ces chaînes-là. Les renommer casse l'affichage
    // sans casser la moindre compilation.
    assert_eq!(Etape::Pack.as_str(), "pack");
    assert_eq!(Etape::Chargeur.as_str(), "chargeur");
    assert_eq!(Etape::Minecraft.as_str(), "minecraft");
    assert_eq!(Etape::Java.as_str(), "java");
    assert_eq!(Etape::NeoForge.as_str(), "neoforge");
    assert_eq!(Etape::Mods.as_str(), "mods");
    assert_eq!(Etape::Verrou.as_str(), "verrou");
}

#[test]
fn le_rapport_muet_accepte_tout_sans_rien_faire() {
    // Il sert aux tests et à tout appelant qui ne veut que le résultat : ce
    // qui compte est qu'il ne panique sur aucune des trois voies.
    let muet = Muet;
    muet.etape(Etape::Mods);
    muet.note("quelque chose");
    muet.telechargement(mc_dl::Avancement::Recus(1_024));
}
