//! La page d'aide.

use mc_pack::source;

pub fn usage() {
    eprintln!(
        "usage :\n  \
         mc-pack install [source] [--locked] [--with-server] [--instance NOM] [--data DIR]\n  \
         mc-pack lock    <manifeste> [--data DIR]\n  \
         mc-pack verify  [source] [--deep] [--data DIR]\n  \
         mc-pack launch  [source] --pseudo NOM [--serveur HOTE[:PORT]] [--memoire MO] [--afficher]\n  \
         mc-pack diagnostic [--incident-test]\n\n\
         « source » est un chemin vers un manifeste, ou une URL.\n  \
         Par défaut : {}",
        source::url_par_defaut()
    );
}
