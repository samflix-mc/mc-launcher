use super::autorisee;
use tauri::Url;

fn url(brut: &str) -> Url {
    Url::parse(brut).expect("URL d'essai valide")
}

/// Les trois origines de l'application, et ce sont bien trois : l'origine du
/// protocole d'actifs n'est pas la même partout. En oublier une donne une
/// fenêtre blanche sur la plateforme oubliée seulement — c'est-à-dire, en
/// pratique, chez quelqu'un d'autre.
#[test]
fn les_origines_de_l_application_passent() {
    for brut in [
        // Linux et macOS.
        "tauri://localhost",
        "tauri://localhost/",
        "tauri://localhost/index.html",
        // Windows.
        "http://tauri.localhost/",
        "http://tauri.localhost/index.html",
        // `tauri dev`, où c'est le serveur d'Angular qui sert la page.
        "http://localhost:1420/",
        "http://localhost:1420/index.html",
    ] {
        assert!(autorisee(&url(brut)), "refusée à tort : {brut}");
    }
}

/// Le routeur emploie `withHashLocation()` : toute la navigation interne se
/// fait par le fragment. Refuser ces URL figerait l'application sur son
/// premier écran.
#[test]
fn la_navigation_par_fragment_passe() {
    for brut in [
        "tauri://localhost/#/spawn",
        "tauri://localhost/index.html#/nouvelles",
        "http://tauri.localhost/#/configuration",
    ] {
        assert!(autorisee(&url(brut)), "refusée à tort : {brut}");
    }
}

/// Ce que le contrôle existe pour empêcher : un lien d'une nouvelle qui ferait
/// naviguer la fenêtre entière vers un site distant. Cette fenêtre est une
/// origine privilégiée où `invoke` est joignable ; un site qui s'y chargerait
/// hériterait du trousseau et du système de fichiers.
#[test]
fn un_site_distant_est_refuse() {
    for brut in [
        "https://exemple.invalid/",
        "http://mc-launcher.ggy.info/",
        "https://modrinth.com/mod/jei",
        "https://mc-heads.net/head/abc",
    ] {
        assert!(!autorisee(&url(brut)), "acceptée à tort : {brut}");
    }
}

/// Un hôte qui COMMENCE par le bon nom n'est pas le bon hôte. C'est l'attaque
/// la plus simple contre une comparaison écrite avec `starts_with`, et elle
/// marche : « tauri.localhost.evil.com » commence bien par « tauri.localhost ».
#[test]
fn un_hote_qui_commence_par_le_bon_nom_est_refuse() {
    for brut in [
        "http://tauri.localhost.evil.invalid/",
        // Un sous-domaine, et non un suffixe : « localhost.evil.invalid »
        // n'appartient pas plus à la machine du joueur que le précédent.
        "http://localhost.evil.invalid:1420/",
        "http://localhost:14200/",
        "https://localhost:1420/",
    ] {
        assert!(!autorisee(&url(brut)), "acceptée à tort : {brut}");
    }
}

/// Les schémas qui ne mènent pas à une page : un `file://` sortirait du
/// protocole d'actifs, un `javascript:` exécuterait du code dans l'origine
/// privilégiée, et un `data:` y chargerait un document arbitraire.
#[test]
fn les_schemas_dangereux_sont_refuses() {
    for brut in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "about:blank",
    ] {
        assert!(!autorisee(&url(brut)), "acceptée à tort : {brut}");
    }
}

/// Le port compte. `http://localhost` sans port n'est pas le serveur de
/// développement : c'est n'importe quel service qui écoute en 80 sur la
/// machine du joueur.
#[test]
fn localhost_sans_le_bon_port_est_refuse() {
    assert!(!autorisee(&url("http://localhost/")));
    assert!(!autorisee(&url("http://localhost:4200/")));
    assert!(!autorisee(&url("http://127.0.0.1:1420/")));
}
