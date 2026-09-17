//! Télécharger ce qu'un tour a retenu, puis lire ce que les jars exigent.
//!
//! Les deux vont ensemble : un jar n'est lisible qu'une fois descendu, et
//! c'est cette lecture qui décide s'il faudra un tour de plus.

use anyhow::Result;
use std::collections::BTreeMap;

use crate::resolve::Registry;
use crate::resolve::file::Cle;
use crate::resolve::inspection::inspect_all;
use crate::resolve::plan::Installed;
use crate::resolve::telechargement::download_all;

pub(super) async fn telecharger_et_lire(
    registry: &Registry,
    chosen: &mut BTreeMap<Cle, Installed>,
    pass: usize,
) -> Result<()> {
    let a_telecharger = chosen
        .values()
        .filter(|m| m.path.as_os_str().is_empty())
        .count();
    let debut = std::time::Instant::now();

    download_all(registry, chosen).await?;
    inspect_all(chosen)?;

    if vaut_d_etre_annonce(a_telecharger) {
        tracing::info!(
            tour = pass,
            jars = a_telecharger,
            duree_ms = debut.elapsed().as_millis(),
            "{a_telecharger} jars téléchargés et analysés en {} ms (tour {pass})",
            debut.elapsed().as_millis()
        );
    }
    Ok(())
}

/// Un tour qui n'a rien téléchargé n'a rien à annoncer.
///
/// La résolution fait plusieurs tours, et les derniers ne descendent souvent
/// aucun jar : ils ne font que relire ce que les précédents ont posé. Annoncer
/// « 0 jars téléchargés en 3 ms » à chacun noierait la ligne qui compte — et ne
/// plus rien annoncer du tout priverait le joueur du seul signe que
/// l'installation avance.
fn vaut_d_etre_annonce(a_telecharger: usize) -> bool {
    a_telecharger > 0
}

#[cfg(test)]
#[path = "descente.test.rs"]
mod tests;
