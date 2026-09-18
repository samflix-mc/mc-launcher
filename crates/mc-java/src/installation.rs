//! Download a Temurin and put it in the right place.

use anyhow::{Context, Result, bail};
use std::path::Path;

use crate::adoptium::{API, Asset, platform, url_assets};
use crate::archive::{extract, single_child};
use crate::locations::{java_exe, managed_home};
use crate::version::{Java, Origin, probe};

/// platform combinations, hence the fallback to the JDK.
///
/// ## The observer, and what it fixes
///
/// This module used to build its own client, without an observer. The
/// consequence showed in the window: the "Java" step would light up, then
/// nothing for a hundred and eighty megabytes — no rate, no bar, no time
/// remaining. It's the only moment of the step that takes a while, and it
/// was the only one nobody narrated.
///
/// `Option` all the same: the command line has no bar to feed, and forcing
/// it to build one would serve no one.
#[tracing::instrument(name = "installation java", skip(runtime_dir, observer))]
pub async fn install(
    major: u32,
    runtime_dir: &Path,
    observer: Option<mc_dl::Observer>,
) -> Result<Java> {
    install_from(API, major, runtime_dir, observer).await
}

/// The same installation, against a given API root.
pub(crate) async fn install_from(
    base: &str,
    major: u32,
    runtime_dir: &Path,
    observer: Option<mc_dl::Observer>,
) -> Result<Java> {
    let (os, arch) = platform()?;
    tokio::fs::create_dir_all(runtime_dir)
        .await
        .with_context(|| format!("creating {}", runtime_dir.display()))?;
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    let dl = match observer {
        Some(observer) => dl.observe(observer),
        None => dl,
    };

    let mut asset = None;
    for image in ["jre", "jdk"] {
        let body = dl.bytes(&url_assets(base, major, os, arch, image)).await?;
        let assets: Vec<Asset> = serde_json::from_slice(&body)
            .with_context(|| format!("unreadable Adoptium response for {image} {major}"))?;
        if let Some(found) = assets.into_iter().find(|a| a.binary.image_type == image) {
            asset = Some(found);
            break;
        }
    }
    let asset =
        asset.with_context(|| format!("Adoptium doesn't publish Java {major} for {os}/{arch}"))?;

    let home = managed_home(runtime_dir, major);
    let archive = runtime_dir.join(&asset.binary.package.name);
    // Adoptium publishes a SHA-256 per package: a JDK is code run with the
    // user's privileges, verifying it is not optional.
    let sum = mc_dl::Checksum::Sha256(asset.binary.package.checksum.clone());
    dl.to_file(
        &asset.binary.package.link,
        &archive,
        mc_dl::Check::Full(&sum),
    )
    .await
    .with_context(|| format!("downloading {}", asset.release_name))?;

    if home.exists() {
        tokio::fs::remove_dir_all(&home).await?;
    }
    // Extraction into a temp directory: the archive contains a root folder
    // named after the version, which we don't want in the final path.
    let staging = runtime_dir.join(format!(".temurin-{major}-extraction"));
    if staging.exists() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    tokio::fs::create_dir_all(&staging).await?;
    extract(&archive, &staging)?;

    let root = single_child(&staging)?;
    tokio::fs::rename(&root, &home)
        .await
        .with_context(|| format!("installing to {}", home.display()))?;
    tokio::fs::remove_dir_all(&staging).await.ok();
    tokio::fs::remove_file(&archive).await.ok();

    // Last check, and the only one that proves anything: the installed
    // binary starts and reports the right version.
    let exe = java_exe(&home);
    let version = probe(&exe)
        .await
        .with_context(|| format!("the Java installed in {} doesn't start", home.display()))?;
    // `!=` and not `<`: the counterpart to `detection.rs`'s `==`. A Temurin
    // newer than requested isn't "good enough", it's the wrong one — and
    // accepting it here would make `ensure` reinstall it endlessly, since
    // `detect` would refuse it on the next launch.
    if version.major != major {
        bail!(
            "Temurin {} installed, but it reports Java {} while {major} is required",
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
