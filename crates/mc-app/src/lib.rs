//! L'interface de Helm, le launcher du réseau samflix-mc.
//!
//! La fenêtre. Elle authentifie le joueur, installe le pack, le vérifie et
//! lance le jeu — chacune de ces étapes étant déjà écrite et testée dans l'un
//! des crates de la racine.
//!
//! ## Ce que ce crate fait, et ce qu'il ne fait pas
//!
//! Il ne contient aucune logique de launcher. L'authentification est celle de
//! `mc-auth`, l'installation celle de `mc-pack`, la journalisation celle de
//! `mc-log`, les chemins ceux de `mc-chemins`. Ce module n'ajoute qu'un pont :
//! des types sérialisables, les commandes que le front appelle, et l'ordre
//! dans lequel tout cela démarre.
//!
//! C'est aussi pourquoi il est hors du périmètre de mutation (voir
//! `default-members` dans le Cargo.toml racine) : muter une enveloppe
//! demanderait à un test de vérifier une délégation, ce que seul un essai de
//! bout en bout — avec un serveur d'affichage — pourrait faire.
//!
//! ## Pourquoi la fenêtre journalise par `mc-log`
//!
//! `tauri-plugin-log` écrirait les jetons tels quels. `mc-log` les censure —
//! `refresh_token`, `access_token` — sur la console, dans le fichier et avant
//! Sentry. Un launcher graphique avale sa sortie standard : le fichier de
//! journal est alors la seule chose qu'un joueur puisse joindre à un rapport,
//! et c'est exactement le moment où il ne faut pas qu'un jeton s'y trouve.

mod chemins;
mod cinematique;
mod commandes;
mod csp;
mod demarrage;
mod fenetres;
// Le launcher sans sa fenêtre, servi sur HTTP. Derrière une feature qui n'est
// pas activée par défaut : `cargo tauri build` ne le compile pas.
#[cfg(feature = "dev-serveur")]
pub mod dev;
mod diagnostic;
mod marque;
mod navigation;
mod phase;
mod recette;
mod suivi;
mod webkit;

/// Monte la fenêtre et rend la main quand elle se ferme.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // En premier, avant le moindre fil : l'appel écrit l'environnement du
    // processus, et ce n'est sûr que tant qu'il est seul à y toucher.
    let dmabuf_desactive = webkit::regler_le_rendu();

    // Avant la fenêtre et avant le journal : `--diagnostic` répond puis sort.
    // C'est ce que la CI lance sur le binaire qu'elle vient de construire pour
    // savoir s'il porte l'environnement qu'on croit — un binaire de production
    // qui se croit « development » irait chercher le pack de dev et ferait
    // entrer les joueurs sur le serveur de dev. Ouvrir une fenêtre pour
    // répondre à cette question demanderait un serveur d'affichage sur un
    // runner qui n'en a pas.
    if diagnostic::demande(std::env::args()) {
        print!("{}", diagnostic::rapport(dmabuf_desactive));
        return;
    }

    // L'ORDRE DE CE QUI SUIT EST LE SUJET, et il se lit mal :
    //
    // 1. `build()` construit l'application SANS ouvrir de fenêtre, mais avec
    //    son résolveur de chemins déjà en place.
    // 2. On pose donc les emplacements ici — avant le journal, qui doit
    //    ouvrir son fichier au bon endroit du premier coup.
    // 3. `mc_log::init` ensuite.
    // 4. `app.run()` enfin, et c'est lui qui crée la fenêtre.
    //
    // Le piège est à l'étape 1 : juste après `build()`,
    // `get_webview_window("main")` rend `None`. Les fenêtres déclarées dans
    // tauri.conf.json sont construites par la fonction libre `setup()`
    // (app.rs:2520-2535), appelée sur `RuntimeRunEvent::Ready`
    // (app.rs:1422-1428), à l'intérieur de `App::run`. C'est pour cela que le
    // hook `.setup()` ci-dessous RESTE : y substituer un appel direct ici ne
    // ferait rien — sans un avertissement — et la fenêtre garderait le titre
    // figé du fichier de configuration.
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // La liste blanche de navigation. Sous forme de greffon parce que
        // `on_navigation` n'existe pas sur `tauri::Builder` : seulement sur un
        // constructeur de webview — or la nôtre est déclarée dans
        // tauri.conf.json et n'existe pas encore ici — ou sur un constructeur
        // de greffon, dont le magasin est consulté pour TOUTE webview
        // (manager/webview.rs:596-602).
        .plugin(navigation::greffon())
        // La recette du build empaqueté, jouée par la fenêtre. Greffon parce
        // qu'il faut poser le collecteur de violations AVANT le document, et
        // qu'une webview déclarée dans tauri.conf.json n'existe pas encore
        // ici. Absent du binaire de production.
        .plugin(recette::greffon())
        // Le compteur d'avancement vit aussi longtemps que la fenêtre : les
        // téléchargements l'incrémentent depuis leurs tâches, la boucle
        // d'émission le lit, et aucune commande ne peut le posséder.
        .manage(commandes::Etat::default())
        // Le hook `setup` fait deux choses, et c'est le seul endroit d'où
        // elles soient possibles : il s'exécute une fois les fenêtres de
        // `tauri.conf.json` réellement construites — ce qui n'est PAS le cas
        // juste après `build()`.
        .setup(|app| {
            use tauri::Manager;

            // Le titre de `tauri.conf.json` est figé dans le fichier ; celui-ci
            // vient de `MC_LAUNCHER_NOM`. Le poser ici évite d'avoir deux
            // endroits à changer pour renommer le launcher, dont un qu'on
            // oublie.
            if let Some(fenetre) = app.get_webview_window("main")
                && let Err(erreur) = fenetre.set_title(marque::nom())
            {
                tracing::warn!(erreur = %erreur, "titre de la fenêtre inchangé");
            }

            // Sans elle, une erreur JavaScript laisserait la fenêtre
            // principale cachée POUR TOUJOURS, derrière un écran de démarrage
            // sans le moindre bouton pour le fermer.
            demarrage::armer_la_garde(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commandes::marque,
            commandes::chemin,
            commandes::statut,
            commandes::connexion,
            commandes::deconnexion,
            // Le front dit quand il a rendu : c'est ce qui referme l'écran de
            // démarrage et montre la fenêtre.
            demarrage::front_pret,
            // Les deux temps de la fenêtre de connexion : on l'ouvre quand la
            // session manque, on la referme quand elle est là.
            fenetres::ouvrir_connexion,
            fenetres::connexion_reussie,
            fenetres::principale_prete,
            // Le geste unique, et ce qu'il faut pour le dessiner.
            commandes::pack::etat_du_pack,
            commandes::pack::jouer,
            commandes::pack::verifier_les_fichiers,
            // Les nouvelles du réseau.
            commandes::nouvelles::nouvelles,
            // Les réglages, et ce que l'écran permet.
            commandes::reglages::reglages,
            commandes::reglages::enregistrer_reglages,
            commandes::reglages::ecran,
            commandes::reglages::ouvrir_dossier,
        ])
        .build(tauri::generate_context!());

    let application = match application {
        Ok(application) => application,
        Err(erreur) => {
            // Un échec de `R::new()` — pas de serveur d'affichage, WebKit
            // indisponible — sortirait sinon sur la seule sortie d'erreur
            // d'une application graphique, que personne ne lit, et sans
            // journal ni Sentry puisque `mc_log::init` n'a pas encore tourné.
            secours_de_demarrage(&erreur);
            panic!("démarrage de la fenêtre : {erreur}");
        }
    };

    // Le résolveur de Tauri est disponible dès `build()` : les crates ne
    // dérivent plus rien toutes seules à partir d'ici.
    chemins::poser(&application);

    // Le garde tient les couches de journalisation ouvertes : le lâcher ici
    // viderait le fichier de son contenu tamponné et couperait Sentry avant
    // même l'affichage de la fenêtre.
    let _journal = mc_log::init("helm");

    chemins::journaliser_la_divergence();

    if dmabuf_desactive {
        // Une fenêtre blanche sous NVIDIA se diagnostique mal ; savoir que le
        // contournement s'est déclenché — ou pas — est la première chose à
        // vérifier dans le journal.
        tracing::info!("pilote NVIDIA détecté, rendu DMA-BUF de WebKit désactivé");
    }

    // Prend une clôture, et ne rend rien : le `expect` d'avant portait sur
    // `build`, pas sur `run`.
    application.run(|_, _| {});
}

/// Écrit la cause d'un démarrage impossible là où les journaux vont
/// d'habitude, puisque `mc-log` n'a pas encore pu s'ouvrir.
///
/// Sans cela, la seule trace d'un poste sans serveur d'affichage serait une
/// panique sur une sortie d'erreur que personne ne lit — et le rapport du
/// joueur se résumerait à « ça ne s'ouvre pas ».
fn secours_de_demarrage(erreur: &tauri::Error) {
    use std::io::Write as _;

    eprintln!("[démarrage] la fenêtre n'a pas pu être construite : {erreur}");

    let journaux = mc_chemins::du_systeme().journaux;
    if std::fs::create_dir_all(&journaux).is_err() {
        return;
    }
    if let Ok(mut fichier) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journaux.join("demarrage-impossible.log"))
    {
        let _ = writeln!(fichier, "la fenêtre n'a pas pu être construite : {erreur}");
    }
}
