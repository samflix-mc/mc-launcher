//! Options du client Sentry.

use crate::environment;
use crate::redact::redact;

use super::dsn;
use super::scrub::{scrub_event, scrub_log_attribute, scrub_value};

/// Initialise le client, ou rend `None` si la télémétrie est coupée.
pub(crate) fn init_sentry(component: &str) -> Option<sentry::ClientInitGuard> {
    let dsn = dsn()?;
    let guard = sentry::init((dsn, options()));

    sentry::configure_scope(|scope| {
        scope.set_tag("composant", component);
    });

    guard.is_enabled().then_some(guard)
}

/// Les options du client, séparées de son ouverture.
///
/// Elles portent l'essentiel des décisions — ce qui est envoyé, ce qui est
/// censuré, ce qui est tu — et `sentry::init` ouvre une connexion vers le
/// projet réel. Les vérifier demandait donc soit d'envoyer pour de bon, soit de
/// les rendre lisibles sans client : c'est ce second choix.
pub(super) fn options() -> sentry::ClientOptions {
    // `ClientOptions` est non exhaustif : il se remplit champ par champ.
    let mut options = sentry::ClientOptions::default();
    // `release_name!()` rendrait le nom du crate qui appelle — soit
    // « mc-log@… » pour tous les binaires, ce qui interdirait de distinguer une
    // version de mc-pack d'une autre. La release nomme le launcher entier ; le
    // composant est porté par une étiquette séparée.
    options.release = Some(format!("mc-launcher@{}", env!("CARGO_PKG_VERSION")).into());
    // Déclaré, jamais déduit du profil de compilation : voir [`environment`].
    options.environment = Some(environment::current().as_str().into());
    // Jamais : ce processus détient des jetons d'authentification.
    options.send_default_pii = false;
    // Ce que `Guard` attend en se détruisant, sur toutes les commandes — y
    // compris celles qui ne touchent jamais au réseau. Porter ce budget à dix
    // secondes faisait payer l'attente à un `verify` hors ligne, qui n'a rien à
    // envoyer que ses propres lignes de journal. Les longues attentes sont
    // demandées là où elles se justifient, par [`flush_incidents`] : sur le
    // chemin d'un plantage, et dans [`send_test_event`].
    options.shutdown_timeout = std::time::Duration::from_secs(2);
    options.attach_stacktrace = true;
    // Le SDK renseigne sinon le nom de la machine. Sur un poste de joueur,
    // c'est une donnée identifiante qui n'apprend rien sur la panne : la chaîne
    // vide neutralise l'intégration qui le renseignerait.
    options.server_name = Some("".into());
    // Dernier filet : tout texte sortant est censuré, y compris ce que des
    // bibliothèques tierces auraient ajouté sans qu'on le sache.
    options.before_send = Some(std::sync::Arc::new(|mut event| {
        scrub_event(&mut event);
        Some(event)
    }));
    options.before_breadcrumb = Some(std::sync::Arc::new(|mut crumb| {
        crumb.message = crumb.message.map(|m| redact(&m));
        for value in crumb.data.values_mut() {
            scrub_value(value);
        }
        Some(crumb)
    }));
    // Les journaux structurés empruntent un canal distinct : `before_send` ne
    // les voit pas. Sans ce second filtre, la censure serait contournée par la
    // voie la plus bavarde de toutes.
    options.before_send_log = Some(std::sync::Arc::new(|mut log| {
        log.body = redact(&log.body);
        // Le SDK ajoute l'adresse du serveur à chaque entrée : sur un poste de
        // joueur, c'est le nom de sa machine.
        log.attributes.remove("server.address");
        for attribute in log.attributes.values_mut() {
            scrub_log_attribute(attribute);
        }
        Some(log)
    }));

    options
}

#[cfg(test)]
#[path = "client.test.rs"]
mod tests;
