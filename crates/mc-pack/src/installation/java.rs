//! Le runtime avec lequel NeoForge sera installé, puis lancé.

use std::sync::Arc;

use anyhow::{Context, Result};

use crate::progression::Rapport;

pub(super) async fn runtime(
    java_major: u32,
    layout: &mc_instance::Layout,
    rapport: &Arc<dyn Rapport>,
) -> Result<mc_java::Java> {
    // L'observateur descend jusqu'à mc-java, et c'est ce qui change tout pour
    // qui regarde la fenêtre : sans lui, l'étape « Java » s'allumait puis ne
    // disait plus rien pendant cent quatre-vingts mégaoctets. Une fenêtre
    // muette pendant deux minutes ne se distingue pas d'une fenêtre plantée.
    //
    // L'observateur n'est nourri que si un runtime doit RÉELLEMENT être posé :
    // quand `detect` en trouve un, `ensure` rend la main sans rien télécharger,
    // et la barre ne bouge pas parce qu'il n'y a rien à raconter.
    let java = mc_java::ensure(
        java_major,
        &layout.runtime(),
        Some(super::observateur(rapport)),
    )
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
