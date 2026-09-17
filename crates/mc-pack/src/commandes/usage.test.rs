use super::usage;
use crate::commandes::essais::environnement;

/// L'aide nomme la source par défaut, et celle-ci dépend de l'environnement du
/// binaire : une préproduction qui annoncerait le pack de production enverrait
/// chercher au mauvais endroit.
///
/// L'aide part sur la sortie d'erreur ; ce que le test retient est qu'elle
/// s'assemble sans paniquer, et que l'adresse annoncée suit l'environnement.
#[test]
fn l_aide_annonce_la_source_de_cet_environnement() {
    let env = environnement("production");
    usage();
    assert_eq!(
        mc_pack::source::url_par_defaut(),
        mc_pack::source::URL_PRODUCTION
    );

    env.poser("preproduction");
    usage();
    assert_eq!(
        mc_pack::source::url_par_defaut(),
        mc_pack::source::URL_PREPRODUCTION
    );

    // Un binaire compilé à la main vise la dev : un pack de dev installé par
    // erreur se corrige d'un --source, l'inverse se remarque moins.
    for local in ["development", "local"] {
        env.poser(local);
        assert_eq!(
            mc_pack::source::url_par_defaut(),
            mc_pack::source::URL_DEVELOPPEMENT,
            "pour « {local} »"
        );
    }
}
