//! L'interface du launcher samflix-mc.
//!
//! Une maquette, pour l'instant : elle sait dire qui est connecté, ouvrir une
//! session Microsoft, l'oublier — et rien de plus. Le bouton « Jouer » existe
//! et ne lance pas le jeu ; c'est délibéré, et il le dit.
//!
//! ## Ce que ce crate fait, et ce qu'il ne fait pas
//!
//! Il ne contient aucune logique de launcher. L'authentification est celle de
//! `mc-auth`, la journalisation celle de `mc-log` — ce module n'ajoute qu'un
//! pont : des types sérialisables, quatre commandes, et un coffre pour le
//! jeton. Tout ce qui viendra ensuite — installation du pack, vérification,
//! lancement — est déjà écrit dans les crates de la racine et n'aura qu'à être
//! appelé de la même façon.
//!
//! ## Pourquoi la fenêtre journalise par `mc-log`
//!
//! `tauri-plugin-log` écrirait les jetons tels quels. `mc-log` les censure —
//! `refresh_token`, `access_token` — sur la console, dans le fichier et avant
//! Sentry. Un launcher graphique avale sa sortie standard : le fichier de
//! journal est alors la seule chose qu'un joueur puisse joindre à un rapport,
//! et c'est exactement le moment où il ne faut pas qu'un jeton s'y trouve.

mod coffre;
mod commandes;

/// Monte la fenêtre et rend la main quand elle se ferme.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Le garde tient les couches de journalisation ouvertes : le lâcher ici
    // viderait le fichier de son contenu tamponné et couperait Sentry avant
    // même l'affichage de la fenêtre.
    let _journal = mc_log::init("samflix-launcher");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commandes::statut,
            commandes::connexion,
            commandes::deconnexion,
            commandes::lancer_jeu,
        ])
        .run(tauri::generate_context!())
        .expect("démarrage de la fenêtre");
}
