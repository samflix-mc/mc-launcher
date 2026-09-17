//! Ce qu'on affiche avant de rendre la main au jeu.

use mc_pack::manifest::Manifest;

use mc_pack::Partie;

/// Hors de portée des tests de mutation : cette fonction n'a d'autre effet que
/// d'écrire sur la sortie standard, et Rust n'offre pas de moyen stable de la
/// relire depuis le processus qui l'émet. Ce qu'elle affiche, en revanche, se
/// vérifie ligne à ligne — c'est [`lignes`], juste en dessous.
#[mutants::skip]
pub(super) fn annoncer(partie: &Partie) {
    for ligne in lignes(partie) {
        println!("{ligne}");
    }
    println!();
}

/// Les lignes du récapitulatif, séparées de leur affichage.
///
/// C'est la dernière chose qu'un joueur lit avant que le jeu ne prenne la
/// main, et la première qu'il recopie quand il demande de l'aide. Chacune
/// répond à une question posée en vrai : quelle instance, quelle version, sous
/// quel nom, où sont les mods, et quel serveur — s'il y en a un.
pub(super) fn lignes(partie: &Partie) -> Vec<String> {
    let instance = &partie.instance;
    let session = &partie.session;

    // La provenance est dite, pas seulement l'adresse : « mc.exemple.fr
    // (production) » laissait croire que l'hôte venait du pack, alors qu'un
    // --serveur peut désigner n'importe quoi. Quelqu'un qui diagnostique une
    // éjection a besoin de savoir lequel des deux il regarde.
    //
    // La clé affichée est celle réellement lue, pas l'environnement du
    // binaire : en « local » c'est l'entrée « development » qui sert, et
    // afficher « local » enverrait chercher dans le manifeste une clé absente.
    let clef = Manifest::environnement_serveur(partie.environnement);
    let serveur = match (&partie.cible, partie.demande_explicite) {
        (Some(hote), true) => format!("  serveur : {hote} — demandé en ligne de commande"),
        (Some(hote), false) => format!(
            "  serveur : {hote} — déclaré par le pack pour « {} »",
            clef.as_str()
        ),
        (None, _) => format!(
            "  serveur : aucun pour « {} » — le jeu s'ouvrira sur le menu",
            clef.as_str()
        ),
    };

    vec![
        format!("Instance « {} »", instance.name),
        format!("  version : {}", partie.version_id),
        format!("  joueur  : {} ({})", session.name, session.uuid),
        format!("  mods    : {}", instance.mods_dir().display()),
        serveur,
    ]
}

#[cfg(test)]
#[path = "annonce.test.rs"]
mod tests;
