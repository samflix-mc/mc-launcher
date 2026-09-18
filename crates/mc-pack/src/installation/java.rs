//! The runtime NeoForge will be installed with, then launched.

use std::sync::Arc;

use anyhow::{Context, Result};

use crate::progress::Report;

pub(super) async fn runtime(
    java_major: u32,
    layout: &mc_instance::Layout,
    report: &Arc<dyn Report>,
) -> Result<mc_java::Java> {
    // The observer goes all the way down to mc-java, and that's what makes
    // all the difference for whoever is watching the window: without it,
    // the "Java" step would light up and then say nothing for a hundred and
    // eighty megabytes. A window silent for two minutes looks no different
    // from a crashed one.
    //
    // The observer is only fed if a runtime REALLY needs to be placed: when
    // `detect` finds one, `ensure` returns without downloading anything,
    // and the bar doesn't move because there's nothing to report.
    let java = mc_java::ensure(java_major, &layout.runtime(), Some(super::observer(report)))
        .await
        .with_context(|| format!("no usable Java {java_major}"))?;

    tracing::info!(
        version = %java.version.full,
        required_major = java_major,
        origin = ?java.origin,
        "Java {} used ({})",
        java.version.full,
        match java.origin {
            mc_java::Origin::Managed => "installed by the launcher",
            mc_java::Origin::System => "system runtime",
        }
    );
    Ok(java)
}
