//! Télécharger un Temurin et le poser au bon endroit.

use anyhow::{Context, Result, bail};
use std::path::Path;

use crate::adoptium::{API, Asset, platform, url_assets};
use crate::archive::{extract, single_child};
use crate::emplacements::{java_exe, managed_home};
use crate::version::{Java, Origin, probe};

/// combinaisons de plateformes, d'où le repli sur le JDK.
///
/// ## L'observateur, et ce qu'il corrige
///
/// Ce module fabriquait son propre client, sans observateur. La conséquence se
/// voyait dans la fenêtre : l'étape « Java » s'allumait, puis plus rien
/// pendant cent quatre-vingts mégaoctets — ni débit, ni barre, ni temps
/// restant. C'est le seul moment de l'étape qui dure, et c'était le seul que
/// personne ne racontait.
///
/// `Option` malgré tout : la ligne de commande n'a pas de barre à nourrir, et
/// lui imposer d'en fabriquer une ne servirait personne.
#[tracing::instrument(name = "installation java", skip(runtime_dir, observateur))]
pub async fn install(
    major: u32,
    runtime_dir: &Path,
    observateur: Option<mc_dl::Observateur>,
) -> Result<Java> {
    install_depuis(API, major, runtime_dir, observateur).await
}

/// La même installation, contre une racine d'API donnée.
pub(crate) async fn install_depuis(
    base: &str,
    major: u32,
    runtime_dir: &Path,
    observateur: Option<mc_dl::Observateur>,
) -> Result<Java> {
    let (os, arch) = platform()?;
    tokio::fs::create_dir_all(runtime_dir)
        .await
        .with_context(|| format!("création de {}", runtime_dir.display()))?;
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    let dl = match observateur {
        Some(observateur) => dl.observe(observateur),
        None => dl,
    };

    let mut asset = None;
    for image in ["jre", "jdk"] {
        let body = dl.bytes(&url_assets(base, major, os, arch, image)).await?;
        let assets: Vec<Asset> = serde_json::from_slice(&body)
            .with_context(|| format!("réponse Adoptium illisible pour {image} {major}"))?;
        if let Some(found) = assets.into_iter().find(|a| a.binary.image_type == image) {
            asset = Some(found);
            break;
        }
    }
    let asset = asset
        .with_context(|| format!("Adoptium ne publie pas de Java {major} pour {os}/{arch}"))?;

    let home = managed_home(runtime_dir, major);
    let archive = runtime_dir.join(&asset.binary.package.name);
    // Adoptium publie un SHA-256 par paquet : un JDK est du code exécuté avec
    // les droits de l'utilisateur, le vérifier n'est pas optionnel.
    let sum = mc_dl::Checksum::Sha256(asset.binary.package.checksum.clone());
    dl.to_file(
        &asset.binary.package.link,
        &archive,
        mc_dl::Check::Full(&sum),
    )
    .await
    .with_context(|| format!("téléchargement de {}", asset.release_name))?;

    if home.exists() {
        tokio::fs::remove_dir_all(&home).await?;
    }
    // Extraction dans un répertoire temporaire : l'archive contient un dossier
    // racine au nom de la version, qu'on ne veut pas dans le chemin final.
    let staging = runtime_dir.join(format!(".temurin-{major}-extraction"));
    if staging.exists() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    tokio::fs::create_dir_all(&staging).await?;
    extract(&archive, &staging)?;

    let root = single_child(&staging)?;
    tokio::fs::rename(&root, &home)
        .await
        .with_context(|| format!("installation vers {}", home.display()))?;
    tokio::fs::remove_dir_all(&staging).await.ok();
    tokio::fs::remove_file(&archive).await.ok();

    // Dernière vérification, et la seule qui prouve quoi que ce soit : le
    // binaire installé démarre et annonce la bonne version.
    let exe = java_exe(&home);
    let version = probe(&exe)
        .await
        .with_context(|| format!("le Java installé dans {} ne démarre pas", home.display()))?;
    // `!=` et non `<` : le pendant du `==` de `detection.rs`. Un Temurin plus
    // récent que demandé n'est pas « assez bon », il est le mauvais — et
    // l'accepter ici ferait que `ensure` le réinstalle sans fin, puisque
    // `detect` le refuserait au lancement suivant.
    if version.major != major {
        bail!(
            "Temurin {} installé, mais il annonce Java {} alors que {major} est exigé",
            asset.release_name,
            version.major
        );
    }

    Ok(Java {
        path: exe,
        version,
        origin: Origin::Managed,
    })
}

#[cfg(test)]
#[path = "installation.test.rs"]
mod tests;
