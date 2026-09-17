use super::{extract, single_child};
use crate::essais::{Arbre, archive_temurin};

/// L'archive contient un dossier racine au nom de la version, qu'on ne veut
/// pas dans le chemin final : c'est lui que `single_child` retrouve.
#[cfg(unix)]
#[test]
fn une_archive_tar_gz_se_depaquette_en_un_seul_repertoire() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("tar");
    let archive = arbre.racine.join("temurin.tar.gz");
    std::fs::write(&archive, archive_temurin("21.0.5+11")).unwrap();
    let vers = arbre.racine.join("extraction");
    std::fs::create_dir_all(&vers).unwrap();

    extract(&archive, &vers).expect("le tar.gz se dépaquette");

    let racine = single_child(&vers).expect("un seul répertoire racine");
    assert!(racine.join("bin").join("java").is_file());
}

#[test]
fn deux_entrees_a_la_racine_sont_une_archive_inattendue() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("deux-entrees");
    std::fs::write(arbre.racine.join("une"), b"").unwrap();
    std::fs::write(arbre.racine.join("deux"), b"").unwrap();

    let erreur = single_child(&arbre.racine).expect_err("deux entrées");
    assert!(format!("{erreur:#}").contains("2 entrées"), "{erreur:#}");
}

#[test]
fn une_archive_vide_est_refusee_aussi() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("vide");
    let erreur = single_child(&arbre.racine).expect_err("aucune entrée");
    assert!(format!("{erreur:#}").contains("0 entrées"), "{erreur:#}");
}

/// Adoptium publie du `.tar.gz` et du `.zip` ; tout autre suffixe signale un
/// changement de leur côté, qu'il vaut mieux voir tout de suite.
#[test]
fn un_format_inconnu_est_refuse_par_son_nom() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("format");
    let archive = arbre.racine.join("temurin.7z");
    std::fs::write(&archive, b"").unwrap();

    let erreur = extract(&archive, &arbre.racine).expect_err("format non géré");
    assert!(format!("{erreur:#}").contains("temurin.7z"), "{erreur:#}");
}

#[test]
fn un_tar_gz_illisible_est_signale_avec_son_nom() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("tar-casse");
    let archive = arbre.racine.join("temurin.tar.gz");
    std::fs::write(&archive, b"ceci n'est pas du gzip").unwrap();

    let erreur = extract(&archive, &arbre.racine).expect_err("gzip invalide");
    assert!(
        format!("{erreur:#}").contains("temurin.tar.gz"),
        "{erreur:#}"
    );
}

/// Une archive ZIP peut contenir des chemins remontants qui écriraient hors du
/// répertoire cible. Un JDK est du code exécuté avec les droits de
/// l'utilisateur : l'entrée doit être refusée, pas écrite ailleurs.
#[test]
fn un_zip_valide_se_depaquette_avec_ses_permissions() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("zip");
    let archive = arbre.racine.join("temurin.zip");

    let mut ecrivain = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    ecrivain.add_directory("jdk-21/bin/", options).unwrap();
    ecrivain.start_file("jdk-21/bin/java", options).unwrap();
    {
        use std::io::Write;
        ecrivain.write_all(b"#!/bin/sh\n").unwrap();
    }
    ecrivain.finish().unwrap();

    let vers = arbre.racine.join("extraction");
    extract(&archive, &vers).expect("le zip se dépaquette");

    let pose = vers.join("jdk-21").join("bin").join("java");
    assert!(pose.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&pose).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "le binaire n'est pas exécutable");
    }
}
