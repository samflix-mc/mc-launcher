//! Télécharger ce qu'un tour a retenu, puis lire ce que les jars exigent.
//!
//! Les deux vont ensemble : un jar n'est lisible qu'une fois descendu, et
//! c'est cette lecture qui décide s'il faudra un tour de plus.

use anyhow::Result;
use std::collections::BTreeMap;

use crate::resolve::file::Cle;
use crate::resolve::inspection::inspect_all;
use crate::resolve::plan::Installed;
use crate::resolve::telechargement::download_all;
use crate::resolve::Registry;

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

    if a_telecharger > 0 {
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
