//! Le launcher sans sa fenêtre, servi sur HTTP.
//!
//! ```sh
//! cargo run -p mc-app --features dev-serveur --bin mc-dev-serveur
//! pnpm --dir web start          # dans un autre terminal
//! ```
//!
//! Le front s'ouvre alors dans un navigateur ordinaire, avec son rechargement
//! à chaud, et parle à ce serveur au lieu de `invoke`.
//!
//! ```sh
//! curl localhost:1421                                   # les scénarios connus
//! curl -X POST localhost:1421/scenario/en-installation  # changer d'état
//! ```
//!
//! Ce binaire n'existe que derrière la feature `dev-serveur`, qui n'est pas
//! activée par défaut : `cargo tauri build` ne le compile pas.

use std::sync::Arc;

use mc_app_lib::dev;

#[tokio::main]
async fn main() {
    let _journal = mc_log::init("mc-dev-serveur");

    let depart = std::env::args()
        .nth(1)
        .and_then(|nom| dev::scenario::Etat::depuis_nom(&nom))
        .unwrap_or(dev::scenario::Etat::Deconnecte);

    let contexte = dev::Contexte::neuf(depart);
    let evenements = contexte.evenements.clone();

    let routeur: dev::http::Routeur = Arc::new(move |requete| {
        let contexte = Arc::clone(&contexte);
        Box::pin(async move { dev::router(contexte, requete).await })
    });

    let port = std::env::var("MC_DEV_PORT")
        .ok()
        .and_then(|valeur| valeur.parse().ok())
        .unwrap_or(dev::PORT);

    println!("Serveur de développement : http://127.0.0.1:{port}");
    println!("Scénario de départ : {depart:?}");
    println!("Les scénarios connus : curl localhost:{port}");

    if let Err(erreur) = dev::http::servir(port, routeur, evenements).await {
        eprintln!("le serveur s'est arrêté : {erreur}");
        std::process::exit(1);
    }
}
