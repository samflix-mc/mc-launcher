//! Journalisation et remontée des incidents.
//!
//! Trois destinations, trois usages distincts :
//!
//! - **la console** — ce qu'un utilisateur voit pendant qu'il attend. Discrète
//!   par défaut (`info`), réglable par `RUST_LOG` ;
//! - **un fichier** — le détail complet (`debug`), avec horodatage et cible.
//!   C'est ce qu'on demande à un joueur de joindre quand quelque chose a raté,
//!   et il est écrit même quand la console est avalée par une interface
//!   graphique ;
//! - **Sentry** — les incidents seuls : paniques et erreurs. Un rapport
//!   automatique évite d'avoir à demander à un joueur de reproduire un bug
//!   qu'il a déjà rencontré.
//!
//! ## Quel niveau pour quoi
//!
//! Sans règle, les niveaux dérivent : tout finit en `info` et le fichier
//! devient illisible, ou tout finit en `debug` et la console ne dit plus rien.
//! La règle tient en une question — **qui a besoin de lire cette ligne ?**
//!
//! | niveau | qui lit | exemples |
//! |---|---|---|
//! | `error` | l'utilisateur, tout de suite | l'installation a échoué, une dépendance est introuvable |
//! | `warn` | celui qui diagnostique après coup | un réessai réseau, une clé refusée, un repli de source |
//! | `info` | le compte rendu de l'exécution | les jalons : version résolue, 7 mods retenus, instance installée |
//! | `debug` | celui qui cherche pourquoi | chaque mod retenu, chaque fichier écrit, chaque durée |
//! | `trace` | le dernier recours | chaque requête HTTP, chaque fichier déjà conforme |
//!
//! La ligne à tenir est celle de `info` : l'enchaînement des `info` d'une
//! exécution doit **raconter ce que le programme a fait**, sans détail inutile
//! et sans trou. C'est ce qu'on relit en premier quand quelque chose a raté, et
//! c'est ce que Sentry conserve.
//!
//! Les opérations qui durent ou qui peuvent échouer sont des **spans**, pas des
//! événements : un span porte sa durée et ses champs, et rattache tout ce qui
//! se produit pendant. Une installation lente se lit alors directement, sans
//! avoir à soustraire des horodatages.
//!
//! ## Journal et affichage ne sont pas la même chose
//!
//! Les commandes écrivent sur la sortie standard un compte rendu mis en forme,
//! destiné à un humain qui attend devant son terminal. Ce n'est pas un journal :
//! ça ne porte ni niveau, ni champ, ni horodatage, et une interface graphique
//! l'afficherait autrement. Les deux coexistent donc volontairement — le même
//! jalon apparaît une fois en texte pour l'utilisateur, une fois en événement
//! structuré pour le diagnostic.
//!
//! ## Ce qui ne part pas
//!
//! Le launcher détient des jetons Microsoft, Xbox Live et Minecraft. La
//! documentation de Sentry propose `send_default_pii: true` ; c'est le contraire
//! qui est fait ici, et chaque texte sortant passe par [`redact`] avant
//! l'envoi. Un rapport d'incident un peu moins précis coûte infiniment moins
//! cher que le jeton d'un compte Microsoft publié dans un tableau de bord.
//!
//! La remontée se coupe par `SAMFLIX_TELEMETRY=0`, et le DSN se remplace par
//! `SENTRY_DSN`. L'environnement de déploiement est déclaré par `SAMFLIX_ENV`
//! et vaut `local` à défaut — voir [`environment`].

mod console;
mod fichier;
mod guard;
mod init;
pub mod environment;
pub mod incidents;
pub mod redact;

pub use environment::Environment;
pub use guard::{log_dir, Guard};
pub use incidents::{capture_game_crash, flush_incidents, send_test_event, telemetry_active};
pub use init::init;
pub use redact::redact;

/// Une couche de journalisation, boxée.
///
/// Leur nombre varie — pas de fichier si le répertoire est en lecture seule,
/// pas de Sentry si la télémétrie est coupée — et les types imbriqués de
/// `tracing-subscriber` ne s'y prêtent pas.
pub(crate) type BoxedLayer =
    Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>;
