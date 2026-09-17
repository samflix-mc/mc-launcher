use super::{Outcome, run};
use crate::launch::commande::Command;

/// `/bin/sh` tient lieu de JVM : ce qu'on vérifie ici n'est pas Minecraft mais
/// la boucle qui lit sa sortie, la réécrit, et en tire les exceptions.
#[cfg(unix)]
fn faux_jeu(script: &str) -> Command {
    Command {
        java: std::path::PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        working_dir: std::env::temp_dir(),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn une_partie_qui_se_termine_bien_ne_remonte_rien() {
    let rapport = run(&faux_jeu("echo 'Stopping worker threads'"))
        .await
        .expect("le processus démarre");

    assert_eq!(rapport.outcome, Outcome::Normal);
    assert!(rapport.errors.is_empty(), "{:?}", rapport.errors);
}

/// Minecraft rattrape beaucoup d'exceptions et continue : ces erreurs-là
/// n'apparaissent nulle part ailleurs, et ce sont souvent elles qui expliquent
/// un comportement signalé bien plus tard.
#[cfg(unix)]
#[tokio::test]
async fn une_exception_relevee_en_cours_de_partie_est_retenue() {
    let rapport = run(&faux_jeu(
        "echo 'java.lang.NullPointerException: rien du tout'; \
         echo '	at net.minecraft.Foo(Foo.java:1)'; \
         echo 'la partie continue'",
    ))
    .await
    .unwrap();

    assert_eq!(rapport.outcome, Outcome::Normal);
    assert_eq!(rapport.errors.len(), 1, "{:?}", rapport.errors);
    assert_eq!(
        rapport.errors[0].exception,
        "java.lang.NullPointerException"
    );
}

/// Les deux flux sont fusionnés : Minecraft écrit sur les deux sans
/// distinction utile, et une exception passée par la sortie d'erreur compte
/// autant que les autres.
#[cfg(unix)]
#[tokio::test]
async fn la_sortie_d_erreur_est_lue_comme_la_sortie_standard() {
    let rapport = run(&faux_jeu(
        "echo 'java.io.IOException: disque plein' >&2; exit 1",
    ))
    .await
    .unwrap();

    assert_eq!(rapport.outcome, Outcome::Failed { code: 1 });
    assert_eq!(rapport.errors.len(), 1, "{:?}", rapport.errors);
}

#[cfg(unix)]
#[tokio::test]
async fn un_java_introuvable_se_dit_avec_son_chemin() {
    let commande = Command {
        java: std::path::PathBuf::from("/usr/lib/jvm/qui-n-existe-pas/bin/java"),
        args: Vec::new(),
        working_dir: std::env::temp_dir(),
    };

    let erreur = run(&commande)
        .await
        .expect_err("aucun binaire à cette place");
    assert!(
        format!("{erreur:#}").contains("qui-n-existe-pas"),
        "{erreur:#}"
    );
}
