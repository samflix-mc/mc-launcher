//! Ce qu'on affiche avant de rendre la main au jeu.

use mc_pack::manifest::Manifest;

use super::preparation::Partie;

pub(super) fn annoncer(partie: &Partie) {
    let instance = &partie.instance;
    let session = &partie.session;
    println!("Instance « {} »", instance.name);
    println!("  version : {}", partie.version_id);
    println!("  joueur  : {} ({})", session.name, session.uuid);
    println!("  mods    : {}", instance.mods_dir().display());

    // La provenance est dite, pas seulement l'adresse : « mc.exemple.fr
    // (production) » laissait croire que l'hôte venait du pack, alors qu'un
    // --serveur peut désigner n'importe quoi. Quelqu'un qui diagnostique une
    // éjection a besoin de savoir lequel des deux il regarde.
    //
    // La clé affichée est celle réellement lue, pas l'environnement du
    // binaire : en « local » c'est l'entrée « development » qui sert, et
    // afficher « local » enverrait chercher dans le manifeste une clé absente.
    let clef = Manifest::environnement_serveur(partie.environnement);
    match (&partie.cible, partie.demande_explicite) {
        (Some(hote), true) => println!("  serveur : {hote} — demandé en ligne de commande"),
        (Some(hote), false) => println!(
            "  serveur : {hote} — déclaré par le pack pour « {} »",
            clef.as_str()
        ),
        (None, _) => println!(
            "  serveur : aucun pour « {} » — le jeu s'ouvrira sur le menu",
            clef.as_str()
        ),
    }
    println!();
}
