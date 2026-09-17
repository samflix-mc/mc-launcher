//! Un faux Temurin sur le disque, et l'archive qui le livre.
//!
//! Deux choses ne se vérifient qu'en les faisant pour de bon : qu'un binaire
//! réponde à `-version` — sur stderr, entre guillemets, dans un format hérité —
//! et qu'une archive Adoptium se dépaquette en un répertoire utilisable. Toutes
//! deux sont fabriquées ici plutôt que simulées.

use std::path::{Path, PathBuf};

/// Sérialise les tests qui écrivent un exécutable puis le lancent.
///
/// Sans cela, la suite échoue par intermittence sur `ETXTBSY` : un autre fil
/// qui duplique le processus hérite un instant du descripteur d'écriture du
/// script, et le noyau refuse d'exécuter un fichier ouvert en écriture. Le
/// verrou ferme cette fenêtre — aucune écriture n'est en cours pendant qu'un
/// autre test lance un processus.
/// Un verrou atomique plutôt qu'un `Mutex` : la moitié des tests concernés
/// sont asynchrones, et tenir un `MutexGuard` à travers un `await` est
/// exactement ce que clippy refuse — à raison, puisque rien ne garantit que la
/// tâche reprenne sur le même fil.
static ATELIER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Atelier;

/// À tenir pendant qu'on fabrique un faux runtime et qu'on l'exécute.
pub(crate) fn atelier() -> Atelier {
    use std::sync::atomic::Ordering;
    while ATELIER
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Atelier
}

impl Drop for Atelier {
    fn drop(&mut self) {
        ATELIER.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Une version majeure qu'aucun système ne publie.
///
/// `candidates` ne se limite pas au répertoire qu'on lui donne : elle sonde
/// aussi `JAVA_HOME`, le `PATH` et `/usr/lib/jvm`. Sur un poste de
/// développement — ou sur un runner de CI, qui livre un JDK récent — un test
/// qui exige « rien n'a été retenu » porterait alors sur le Java du poste et
/// non sur ce que le test a posé. Demander une version que personne ne fournit
/// est la seule façon d'écarter ces candidats sans toucher au code qu'on
/// vérifie.
pub(crate) const MAJEUR_INTROUVABLE: u32 = 999;

pub(crate) struct Arbre {
    pub(crate) racine: PathBuf,
}

impl Arbre {
    pub(crate) fn neuf(nom: &str) -> Arbre {
        let racine = std::env::temp_dir().join(format!(
            "mc-java-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Arbre { racine }
    }
}

impl Drop for Arbre {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

/// Écrit un exécutable qui répond à `-version` comme le fait une JVM.
///
/// La sortie part sur stderr et le numéro est entre guillemets : c'est le
/// format qu'analyse `probe`, et il n'a rien d'évident.
#[cfg(unix)]
pub(crate) fn faux_java(chemin: &Path, version: &str) {
    ecrire_executable(
        chemin,
        &format!(
            "#!/bin/sh\n\
             echo 'openjdk version \"{version}\"' >&2\n\
             echo 'OpenJDK Runtime Environment (build {version})' >&2\n"
        ),
    );
}

/// Un exécutable qui démarre mais ne dit rien d'exploitable : le cas d'un
/// paquet à moitié désinstallé, qu'il faut écarter sans s'arrêter dessus.
#[cfg(unix)]
pub(crate) fn java_muet(chemin: &Path) {
    ecrire_executable(chemin, "#!/bin/sh\nexit 0\n");
}

#[cfg(unix)]
fn ecrire_executable(chemin: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    std::fs::write(chemin, script).unwrap();
    // 0o700 et non 0o755 : le seul à devoir lancer ce script est le processus
    // qui vient de l'écrire. Lui donner les droits du groupe et des autres ne
    // servirait rien et poserait, dans un répertoire temporaire partagé, un
    // exécutable que n'importe quel compte du poste pourrait remplacer entre
    // son écriture et son lancement.
    std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// Fabrique l'archive que publie Adoptium : un répertoire racine au nom de la
/// version, contenant `bin/java`.
#[cfg(unix)]
pub(crate) fn archive_temurin(version: &str) -> Vec<u8> {
    let atelier = Arbre::neuf(&format!("atelier-{version}"));
    let racine = atelier.racine.join(format!("jdk-{version}"));
    faux_java(&racine.join("bin").join("java"), version);

    let mut paquet = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    paquet
        .append_dir_all(format!("jdk-{version}"), &racine)
        .unwrap();
    paquet.into_inner().unwrap().finish().unwrap()
}

/// La réponse d'Adoptium pour un binaire donné, telle qu'elle est désérialisée.
pub(crate) fn reponse_adoptium(image: &str, lien: &str, nom: &str, sha256: &str) -> String {
    format!(
        r#"[{{"release_name":"jdk-21.0.5+11",
             "binary":{{"image_type":"{image}",
                        "package":{{"link":"{lien}","name":"{nom}","checksum":"{sha256}"}}}}}}]"#
    )
}

pub(crate) fn sha256(octets: &[u8]) -> String {
    mc_dl::Checksum::Sha256(String::new()).of(octets)
}
