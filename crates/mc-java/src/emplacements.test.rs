use super::{candidates, default_runtime_dir, java_exe, managed_home};
use std::path::Path;

/// Le runtime installé est dédié au launcher : il vit dans son répertoire de
/// données, n'est pas ajouté au `PATH`, et ne touche pas au Java du système.
#[test]
fn le_runtime_du_launcher_vit_sous_ses_propres_donnees() {
    assert!(default_runtime_dir().ends_with("runtime"));
    assert!(default_runtime_dir().starts_with(mc_chemins::courants().donnees));
}

#[test]
fn chaque_version_majeure_a_son_repertoire() {
    let dir = Path::new("/data/runtime");
    assert_eq!(managed_home(dir, 21), Path::new("/data/runtime/temurin-21"));
    assert_ne!(managed_home(dir, 21), managed_home(dir, 17));
}

#[cfg(unix)]
#[test]
fn l_executable_se_trouve_sous_bin() {
    assert_eq!(
        java_exe(Path::new("/data/runtime/temurin-21")),
        Path::new("/data/runtime/temurin-21/bin/java")
    );
}

/// Le runtime géré passe en premier : s'il est là, c'est le launcher qui l'a
/// installé et vérifié, inutile de sonder le système.
///
/// L'environnement n'est pas manipulé ici : ce crate lance des sous-processus,
/// et poser une variable pendant qu'un autre fil duplique le processus est
/// exactement la course qu'on ne veut pas dans une suite de tests.
#[test]
fn le_runtime_gere_ouvre_toujours_la_liste() {
    let liste = candidates(Path::new("/data/runtime"), 21);
    assert_eq!(liste[0], java_exe(Path::new("/data/runtime/temurin-21")));
}

/// Tous les candidats désignent un exécutable `java`, jamais un répertoire :
/// la détection les passe à `probe`, qui les exécute.
#[test]
fn chaque_candidat_designe_un_executable_java() {
    for candidat in candidates(Path::new("/data/runtime"), 21) {
        let nom = candidat.file_name().unwrap().to_string_lossy().to_string();
        assert!(nom == "java" || nom == "java.exe", "{}", candidat.display());
    }
}
