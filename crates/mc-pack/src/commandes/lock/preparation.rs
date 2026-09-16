//! Le manifeste sur lequel `lock` a le droit de travailler.

use std::path::PathBuf;

use anyhow::{Result, bail};

use mc_pack::manifest::Manifest;
use mc_pack::source::Source;

pub(super) fn manifeste_a_verrouiller(source: &Source) -> Result<(PathBuf, Manifest)> {
    // Résoudre produit un verrou, et un verrou doit se poser quelque part.
    // Un pack publié n'offre pas cet endroit — et n'en a pas besoin : il
    // arrive déjà verrouillé, c'est justement ce qui en fait un pack publié.
    let Some(manifest_path) = source.local_path() else {
        bail!(
            "lock travaille sur un manifeste à éditer : donner un chemin.\n\
             Le pack publié est déjà verrouillé — c'est mc-content qui le résout."
        );
    };
    let manifest = Manifest::load(manifest_path)?;

    // C'est ici, et nulle part ailleurs, qu'une clé de « servers » illisible se
    // refuse. La lecture du manifeste ne peut pas s'en charger : elle s'applique
    // aussi au pack téléchargé, et un binaire qui refuserait un environnement
    // inconnu de lui s'arrêterait le jour où mc-content en déclare un de plus.
    // lock est l'inverse — la commande qu'on lance avant de publier, sur le
    // fichier qu'on vient d'écrire, avec l'auteur devant l'écran.
    let problemes = manifest.problemes_de_serveurs();
    if !problemes.is_empty() {
        bail!(
            "{} : des clés de « servers » ne seraient jamais lues —\n  {}",
            manifest_path.display(),
            problemes.join("\n  ")
        );
    }

    Ok((manifest_path.to_path_buf(), manifest))
}
