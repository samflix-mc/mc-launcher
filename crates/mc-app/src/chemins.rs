//! Imposer aux crates les emplacements que Tauri connaît.
//!
//! ## Le problème, et pourquoi il ne se voit pas
//!
//! Les crates du dépôt dérivent leurs chemins avec `std::env::var_os` ; le
//! résolveur de Tauri passe par la crate `dirs`. Les deux suivent la même
//! convention, mais pas les mêmes règles de repli : un `XDG_DATA_HOME`
//! relatif, un `HOME` absent, un profil Windows itinérant les font diverger.
//!
//! Deux arborescences pour les mêmes données, ce sont huit cents mégaoctets
//! téléchargés une seconde fois dans un répertoire voisin, et une session
//! qu'on ne retrouve pas. Rien ne le signalerait : les deux moitiés du
//! programme fonctionneraient, chacune de son côté.
//!
//! ## Pourquoi c'est possible juste après `build()`
//!
//! `build()` rend une `App`, qui implémente `Manager` : `app.path()` répond
//! dès cet instant, parce que `register_core_plugins()` a installé le
//! `PathResolver` dans l'état géré À L'INTÉRIEUR de `build()`
//! (`tauri/src/app.rs:2398`, `path/plugin.rs:240-246`). Il n'y a pas à
//! attendre le hook `setup`, ni la boucle d'événements.
//!
//! Ce qui n'existe PAS encore à cet instant, en revanche, c'est la fenêtre :
//! celles de `tauri.conf.json` sont construites par la fonction libre
//! `setup()` (`app.rs:2520-2535`), appelée sur `RuntimeRunEvent::Ready`
//! (`app.rs:1422-1428`), à l'intérieur de `App::run`. C'est pour cela que le
//! hook `setup` du builder reste en place.
//!
//! ## Ce que fait la déduction, et ce qu'elle ne fait pas
//!
//! On prend de Tauri les RACINES, et on laisse `mc-chemins` en DÉDUIRE
//! l'arborescence. C'est la seule répartition qui tienne : les racines peuvent
//! diverger d'une plateforme à l'autre — c'est leur métier — mais la
//! déduction, elle, n'est que des `join`, et la même des deux côtés. Prendre
//! aussi le `app_log_dir()` de Tauri romprait cette symétrie : il range sous
//! `~/Library/Logs` sous macOS, donc hors de ce qu'il suffit de supprimer pour
//! repartir de zéro.

use tauri::{App, Manager};

/// Pose les emplacements de Tauri pour tout le processus.
///
/// À appeler juste après `build()`, AVANT `mc_log::init` : le journal ouvre un
/// fichier, et il doit l'ouvrir au bon endroit du premier coup.
pub fn poser(app: &App) {
    let resolveur = app.path();

    // `data_dir()`, `config_dir()` et `temp_dir()` rendent un `Result`
    // (`Error::UnknownPath`). Il n'y a rien à rattraper : sans répertoire
    // personnel, il n'y a nulle part où installer huit cents mégaoctets. On
    // retombe alors sur ce que l'environnement dit, qui ne sera pas meilleur,
    // mais qui au moins ne sera pas vide.
    let secours = mc_chemins::du_systeme();

    let racine =
        |resolu: Result<std::path::PathBuf, tauri::Error>, defaut: &std::path::Path, quoi: &str| {
            match resolu {
                Ok(chemin) => chemin.join(mc_chemins::SEGMENT),
                Err(erreur) => {
                    // Avant `mc_log::init` : `eprintln!` est tout ce qu'on a, et
                    // dans une application graphique il n'ira nulle part. C'est
                    // assumé — le cas est celui d'un système sans répertoire
                    // personnel, où rien ne marchera de toute façon.
                    eprintln!("[chemins] {quoi} introuvable ({erreur}), repli sur l'environnement");
                    defaut.to_path_buf()
                }
            }
        };

    // `data_dir()` et NON `app_data_dir()`, et c'est la ligne la plus
    // dangereuse du fichier.
    //
    // `app_data_dir()` rend `<data>/mc.samflix.launcher`, d'après
    // l'`identifier` de tauri.conf.json. Y joindre notre segment donnerait
    // `~/.local/share/mc.samflix.launcher/samflix-mc` — un chemin NEUF, à côté
    // du `~/.local/share/samflix-mc` où huit cents mégaoctets et la session
    // d'un joueur existent déjà. Le launcher se croirait sur un poste vierge,
    // retéléchargerait tout, et laisserait l'ancienne arborescence orpheline
    // sur le disque sans un mot.
    //
    // On prend donc les racines NUES et l'on y joint le segment nous-mêmes :
    // le résultat est, au caractère près, ce que les crates calculaient déjà.
    // C'est la décision « aucune migration ».
    let bases = mc_chemins::Bases {
        donnees: racine(resolveur.data_dir(), &secours.donnees, "données"),
        config: racine(resolveur.config_dir(), &secours.config, "config"),
        temporaire: racine(resolveur.temp_dir(), &secours.temporaire, "temporaire"),
    };

    let voulus = mc_chemins::depuis_bases(bases);

    // La comparaison, et la seule raison pour laquelle elle est ici : si les
    // deux résolveurs divergent un jour, il faut l'apprendre par un journal
    // plutôt que par un joueur qui a téléchargé le pack deux fois. Le journal
    // n'est pas encore ouvert — c'est `mc_log::init` qui suit — donc on garde
    // la divergence pour la dire juste après.
    let divergence = (voulus.donnees != secours.donnees).then(|| {
        format!(
            "Tauri range les données sous {} là où l'environnement dit {}",
            voulus.donnees.display(),
            secours.donnees.display()
        )
    });

    if let Err(erreur) = voulus.creer() {
        eprintln!("[chemins] création impossible ({erreur})");
    }

    if mc_chemins::poser(voulus).is_err() {
        // Une seconde pose est un bogue de séquencement : quelque chose a
        // appelé `courants()` et posé avant nous.
        eprintln!("[chemins] les emplacements étaient déjà posés");
    }

    if let Some(message) = divergence {
        // Après la pose : la trace part dans le journal que la pose vient de
        // situer, et non dans celui qu'on aurait ouvert au mauvais endroit.
        DIVERGENCE.set(message).ok();
    }
}

/// Ce que la pose a constaté, à journaliser une fois `mc-log` ouvert.
static DIVERGENCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Journalise ce que la pose a constaté. À appeler après `mc_log::init`.
pub fn journaliser_la_divergence() {
    if let Some(message) = DIVERGENCE.get() {
        tracing::warn!(
            "{message} — les deux résolveurs ne coïncident pas ; \
             les crates suivent Tauri"
        );
    }
}
