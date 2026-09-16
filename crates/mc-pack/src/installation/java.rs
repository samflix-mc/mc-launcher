//! Le runtime avec lequel NeoForge sera installé, puis lancé.

use anyhow::{Context, Result};

pub(super) async fn runtime(java_major: u32, layout: &mc_instance::Layout) -> Result<mc_java::Java> {
    let java = mc_java::ensure(java_major, &layout.runtime())
        .await
        .with_context(|| format!("aucun Java {java_major} utilisable"))?;
    tracing::info!(
        version = %java.version.full,
        majeur_exige = java_major,
        origine = ?java.origin,
        "Java {} utilisé ({})",
        java.version.full,
        match java.origin {
            mc_java::Origin::Managed => "installé par le launcher",
            mc_java::Origin::System => "runtime du système",
        }
    );
    Ok(java)
}
