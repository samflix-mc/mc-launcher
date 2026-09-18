//! Résoudre le pack et figer le résultat dans un verrou.

mod preparation;
mod rapport;
mod resolution;

use anyhow::Result;
use mc_pack::lockfile::{LockedLoader, Lockfile};
use mc_pack::source::Source;

/// Résout et écrit le verrou sans installer le jeu.
///
/// C'est ce qu'on lance en revue : le verrou montre les versions retenues et
/// les dépendances ajoutées, sans attendre le téléchargement de huit cents
/// mégaoctets d'assets.
/// Hors de portée des tests de mutation : cette commande résout le pack en
/// ligne, écrit le verrou et l'annonce. Chacune de ces trois parties est
/// vérifiée séparément — la résolution chez mc-mods, l'écriture du verrou chez
/// `Lockfile`, le rapport par sa propre suite.
#[mutants::skip]
pub async fn lock(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let (manifest_path, manifest) = preparation::manifeste_a_verrouiller(source)?;
    let (plan, neoforge_version) = resolution::resoudre(&manifest, options).await?;

    let lock_path = Lockfile::path_for(&manifest_path);
    let previous = lock_path
        .is_file()
        .then(|| Lockfile::load(&lock_path))
        .transpose()?;
    let lock = Lockfile::from_plan(
        &manifest,
        LockedLoader {
            kind: manifest.loader.kind.clone(),
            version: neoforge_version,
        },
        // Et non « manifest.java.unwrap_or(21) ». Le verrou est ce qui fait
        // foi pour l'installation ET pour la vérification de Java à chaque
        // lancement : y écrire une constante revenait à figer un chiffre
        // deviné là où l'installation, elle, interrogeait Mojang. Les deux
        // pouvaient diverger, et c'est le verrou qui avait tort.
        //
        // Exactement la même expression qu'à l'étape 4 de l'installation, mot
        // pour mot — y compris le passage par `Manifest::java_major`, qui
        // porte la règle de priorité du manifeste. La remplacer par
        // `java_exige` seul ferait perdre à `lock` la faculté d'imposer une
        // majeure, que `manifest/lecture.demandes.test.rs` verrouille.
        manifest.java_major(
            mc_instance::vanilla::java_exige(
                &manifest.minecraft,
                &mc_dl::Downloader::new(mc_dl::USER_AGENT)?,
            )
            .await?,
        ),
        &plan,
    );
    lock.save(&lock_path)?;

    // Le verrou est le produit de la commande : sa position et ce qui a bougé
    // sont ce qu'on cherchera dans le journal si une revue surprend.
    let changements = previous.as_ref().map(|p| lock.diff(p).len()).unwrap_or(0);
    tracing::info!(
        verrou = %lock_path.display(),
        changements,
        nouveau = previous.is_none(),
        "Verrou écrit dans {} ({})",
        lock_path.display(),
        match (previous.is_none(), changements) {
            (true, _) => "nouveau".to_string(),
            (false, 0) => "inchangé".to_string(),
            (false, n) => format!("{n} changements"),
        }
    );
    rapport::annoncer(&lock, &lock_path, previous.as_ref());
    Ok(())
}
