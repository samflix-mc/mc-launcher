use super::{run_installer, tail};

#[test]
fn la_queue_garde_les_dernieres_lignes() {
    assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail("a", 5), "a");
    assert_eq!(tail("", 5), "");
}

/// `/bin/echo` tient lieu de JVM : ce qui est vérifié n'est pas l'installateur
/// NeoForge mais la façon dont on l'appelle et dont on lit son verdict.
#[cfg(unix)]
#[tokio::test]
async fn un_installateur_qui_reussit_ne_dit_rien() {
    let dir = std::env::temp_dir();
    run_installer(
        std::path::Path::new("installateur.jar"),
        "--install-client",
        &dir,
        std::path::Path::new("/bin/echo"),
    )
    .await
    .expect("code de sortie nul");
}

/// L'installateur écrit son diagnostic utile sur stdout et les traces sur
/// stderr ; les deux sont nécessaires pour comprendre un échec, et un joueur
/// n'ira pas les chercher lui-même.
#[cfg(unix)]
#[tokio::test]
async fn un_echec_rapporte_les_deux_sorties() {
    let script = std::env::temp_dir().join(format!("mc-neoforge-echec-{}", std::process::id()));
    std::fs::write(
        &script,
        "#!/bin/sh\necho 'Failed to install: mauvaise version'\necho 'java.io.IOException' >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let erreur = run_installer(
        std::path::Path::new("installateur.jar"),
        "--install-client",
        &std::env::temp_dir(),
        &script,
    )
    .await
    .expect_err("code de sortie non nul");
    std::fs::remove_file(&script).ok();

    let texte = format!("{erreur:#}");
    assert!(texte.contains("Failed to install"), "{texte}");
    assert!(texte.contains("java.io.IOException"), "{texte}");
}

#[cfg(unix)]
#[tokio::test]
async fn un_java_introuvable_nomme_l_installateur() {
    let erreur = run_installer(
        std::path::Path::new("/cache/neoforge-21.1.250-installer.jar"),
        "--install-client",
        &std::env::temp_dir(),
        std::path::Path::new("/usr/lib/jvm/absent/bin/java"),
    )
    .await
    .expect_err("aucun binaire à cette place");

    assert!(
        format!("{erreur:#}").contains("neoforge-21.1.250-installer.jar"),
        "{erreur:#}"
    );
}
