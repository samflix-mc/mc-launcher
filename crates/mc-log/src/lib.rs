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
//! ## Ce qui ne part pas
//!
//! Le launcher détient des jetons Microsoft, Xbox Live et Minecraft. La
//! documentation de Sentry propose `send_default_pii: true` ; c'est le contraire
//! qui est fait ici, et chaque texte sortant passe par [`redact`] avant
//! l'envoi. Un rapport d'incident un peu moins précis coûte infiniment moins
//! cher que le jeton d'un compte Microsoft publié dans un tableau de bord.
//!
//! La remontée se coupe par `SAMFLIX_TELEMETRY=0`, et le DSN se remplace par
//! `SENTRY_DSN`.

pub mod redact;

pub use redact::redact;

use std::path::{Path, PathBuf};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Projet Sentry du launcher.
///
/// Un DSN n'est pas un secret : il ne permet que d'écrire des événements, et
/// tout client de bureau embarque le sien. Il reste remplaçable par
/// `SENTRY_DSN`, ce qui permet de router les incidents d'un déploiement
/// particulier ailleurs.
const DEFAULT_DSN: &str = "https://5c97a3f2d24e9faf2a5f099a8c5a3a80@o4504715328552960.ingest.us.sentry.io/4512093170434048";

/// Journaux conservés, en jours.
///
/// Assez pour qu'un joueur retrouve la trace d'un incident de la semaine, pas
/// assez pour que le répertoire grossisse indéfiniment.
const KEEP_DAYS: u64 = 14;

/// À garder vivant aussi longtemps que le programme tourne.
///
/// Sa destruction vide la file d'écriture du fichier puis laisse à Sentry le
/// temps d'envoyer ce qui reste. Le lâcher tout de suite perdrait précisément
/// les derniers messages — ceux qui décrivent la sortie.
pub struct Guard {
    _sentry: Option<sentry::ClientInitGuard>,
    _file: Option<tracing_appender::non_blocking::WorkerGuard>,
    pub log_file: Option<PathBuf>,
}

impl Guard {
    /// Chemin du journal, à citer quand quelque chose échoue.
    pub fn log_path(&self) -> Option<&Path> {
        self.log_file.as_deref()
    }
}

/// Répertoire des journaux.
pub fn log_dir() -> PathBuf {
    mc_dl::data_dir().join("logs")
}

/// La remontée d'incidents est-elle active dans cette exécution ?
pub fn telemetry_active() -> bool {
    dsn().is_some()
}

/// Envoie un incident de test et renvoie son identifiant.
///
/// Sert à répondre à « est-ce que ça remonte vraiment ? » sans avoir à
/// provoquer une vraie panique. L'identifiant rendu est celui à chercher dans
/// le tableau de bord : si les deux correspondent, la chaîne entière — envoi,
/// réseau, projet, censure — est vérifiée.
pub fn send_test_event() -> (sentry::types::Uuid, bool) {
    // Les deux canaux passent par des routes différentes et des filtres
    // différents : les tester ensemble évite de croire l'un fonctionnel parce
    // que l'autre l'est.
    tracing::info!(
        canal = "journaux structurés",
        composant = "mc-log",
        // Deux faux jetons, et la différence entre les deux est tout l'intérêt
        // du test.
        //
        // Le premier contient « access_token » : Sentry le filtre lui-même,
        // côté serveur, et le rend en « [Filtered] ». Il ne prouve donc rien
        // sur notre propre censure — un premier essai s'y était laissé prendre.
        //
        // Le second est un JWT nu, qu'aucune règle serveur ne reconnaît. S'il
        // ressort en « [secret] », c'est `before_send_log` qui a agi ; s'il
        // ressort en clair, notre filtre ne fonctionne pas.
        avec_mot_cle = "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA",
        jeton_nu = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdEp3dE51",
        "ligne de journal de test"
    );

    let id = sentry::capture_message(
        "incident de test émis par mc-pack diagnostic --incident-test",
        sentry::Level::Info,
    );
    // L'envoi est asynchrone : sans cette attente, le processus se terminerait
    // avant que la requête ne parte. Le retour dit si la file s'est vidée —
    // c'est la différence entre « un identifiant a été tiré » et « l'incident
    // est parti ».
    let flushed = sentry::Hub::current()
        .client()
        .map(|client| client.flush(Some(std::time::Duration::from_secs(10))))
        .unwrap_or(false);
    (id, flushed)
}

/// La remontée d'incidents est-elle autorisée ?
///
/// Opt-out explicite : une télémétrie qu'on ne peut pas couper n'est pas une
/// télémétrie, c'est une surveillance.
fn telemetry_enabled() -> bool {
    match std::env::var("SAMFLIX_TELEMETRY") {
        Ok(v) => !matches!(v.trim(), "0" | "off" | "false" | "no" | "non"),
        Err(_) => true,
    }
}

fn dsn() -> Option<String> {
    if !telemetry_enabled() {
        return None;
    }
    let configured = std::env::var("SENTRY_DSN").unwrap_or_else(|_| DEFAULT_DSN.to_string());
    let configured = configured.trim().to_string();
    (!configured.is_empty()).then_some(configured)
}

/// Met en place la journalisation pour un composant donné.
///
/// `component` nomme le binaire — il préfixe le fichier de journal et étiquette
/// les incidents, ce qui permet de distinguer un échec d'installation d'un
/// échec d'authentification sans ouvrir le rapport.
pub fn init(component: &str) -> Guard {
    // Avant Sentry : celui-ci chaîne son gestionnaire par-dessus l'existant.
    // Posé après, le nôtre le remplacerait et plus aucune panique ne serait
    // rapportée.
    install_panic_hook();
    let sentry_guard = init_sentry(component);
    let (file_layer, file_guard, log_file) = file_layer(component);

    // Couches boxées : leur nombre varie — pas de fichier si le répertoire est
    // en lecture seule, pas de Sentry si la télémétrie est coupée — et les
    // types imbriqués de `tracing-subscriber` ne s'y prêtent pas.
    let mut layers: Vec<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> = Vec::new();

    // La console montre l'essentiel ; le fichier garde tout. RUST_LOG règle la
    // première sans toucher au second, pour qu'un utilisateur qui augmente la
    // verbosité n'ait pas à relancer l'opération qui a échoué.
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,reqwest=warn,rustls=warn"));

    layers.push(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
            .with_writer(Redacting(std::io::stderr as fn() -> std::io::Stderr))
            .with_filter(console_filter)
            .boxed(),
    );

    if let Some(layer) = file_layer {
        layers.push(layer);
    }

    // Trois traitements distincts, selon ce qu'un niveau signifie :
    //
    // - `Event` ouvre un incident. Réservé aux erreurs, sans quoi le tableau de
    //   bord se remplit de bruit et plus personne ne le regarde ;
    // - `Log` alimente les journaux structurés, cherchables et lisibles à côté
    //   de l'incident correspondant ;
    // - `Breadcrumb` raconte ce qui a précédé, et n'est envoyé qu'attaché à un
    //   incident — donc gratuit tant que rien n'échoue.
    //
    // `debug` reste hors des journaux structurés : c'est le niveau du fichier,
    // des milliers de lignes par installation, et l'envoyer coûterait un quota
    // pour un détail qu'on lit de toute façon en local.
    if sentry_guard.is_some() {
        layers.push(
            sentry_tracing::layer()
                .event_filter(|meta| match *meta.level() {
                    tracing::Level::ERROR => {
                        sentry_tracing::EventFilter::Event | sentry_tracing::EventFilter::Log
                    }
                    tracing::Level::WARN | tracing::Level::INFO => {
                        sentry_tracing::EventFilter::Breadcrumb | sentry_tracing::EventFilter::Log
                    }
                    tracing::Level::DEBUG => sentry_tracing::EventFilter::Breadcrumb,
                    tracing::Level::TRACE => sentry_tracing::EventFilter::Ignore,
                })
                .boxed(),
        );
    }

    tracing_subscriber::registry().with(layers).init();

    if let Some(path) = &log_file {
        tracing::debug!(fichier = %path.display(), "journal ouvert");
    }

    Guard {
        _sentry: sentry_guard,
        _file: file_guard,
        log_file,
    }
}

/// Remplace l'affichage par défaut d'une panique.
///
/// Le gestionnaire standard de Rust écrit le message de panique directement sur
/// la sortie d'erreur, sans passer par `tracing` : il échappe donc à la censure
/// et au fichier de journal. Deux conséquences, toutes deux constatées avant
/// d'écrire ceci — un jeton présent dans un message de panique s'affichait en
/// clair dans le terminal, et la panique restait absente du fichier qu'on
/// demande justement de joindre.
///
/// Le message part en `warn` et non en `error` : l'incident lui-même est
/// rapporté par le gestionnaire de Sentry, avec sa trace d'appels complète, et
/// un `error` ici le ferait remonter une seconde fois.
fn install_panic_hook() {
    // Le gestionnaire d'origine n'est délibérément pas rappelé : il réécrirait
    // le message non censuré sur la sortie d'erreur, ce qui est précisément ce
    // qu'on vient d'éviter.
    std::panic::set_hook(Box::new(move |info| {
        let message = redact(&info.to_string());
        eprintln!("\n{message}");
        eprintln!("(relancer avec RUST_BACKTRACE=1 pour la trace d'appels)");
        tracing::warn!(panique = %message, "le programme s'est arrêté sur une panique");
    }));
}

fn init_sentry(component: &str) -> Option<sentry::ClientInitGuard> {
    let dsn = dsn()?;

    // `ClientOptions` est non exhaustif : il se remplit champ par champ.
    let mut options = sentry::ClientOptions::default();
    // `release_name!()` rendrait le nom du crate qui appelle — soit
    // « mc-log@… » pour tous les binaires, ce qui interdirait de distinguer une
    // version de mc-pack d'une autre. La release nomme le launcher entier ; le
    // composant est porté par une étiquette séparée.
    options.release = Some(format!("mc-launcher@{}", env!("CARGO_PKG_VERSION")).into());
    options.environment = Some(if cfg!(debug_assertions) {
        "development".into()
    } else {
        "production".into()
    });
    // Jamais : ce processus détient des jetons d'authentification.
    options.send_default_pii = false;
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

    let guard = sentry::init((dsn, options));

    sentry::configure_scope(|scope| {
        scope.set_tag("composant", component);
    });

    guard.is_enabled().then_some(guard)
}

/// Censure un événement de bout en bout.
fn scrub_event(event: &mut sentry::protocol::Event<'static>) {
    if let Some(message) = event.message.take() {
        event.message = Some(redact(&message));
    }
    for exception in &mut event.exception.values {
        exception.value = exception.value.as_deref().map(redact);
        if let Some(stacktrace) = &mut exception.stacktrace {
            for frame in &mut stacktrace.frames {
                frame.filename = frame.filename.as_deref().map(redact);
                frame.abs_path = frame.abs_path.as_deref().map(redact);
                for value in frame.vars.values_mut() {
                    scrub_value(value);
                }
            }
        }
    }
    for value in event.extra.values_mut() {
        scrub_value(value);
    }
    for tag in event.tags.values_mut() {
        *tag = redact(tag);
    }
}

/// Censure un attribut de journal structuré.
///
/// Les champs d'un événement `tracing` deviennent des attributs : un
/// `tracing::info!(url = %url, ...)` les expose tels quels. Seules les chaînes
/// peuvent porter un secret ; les nombres et booléens sont laissés intacts,
/// puisqu'ils restent utiles au tri et au filtrage.
fn scrub_log_attribute(attribute: &mut sentry::protocol::LogAttribute) {
    use sentry::protocol::Value;
    if let Value::String(text) = &attribute.0 {
        attribute.0 = Value::String(redact(text));
    }
}

/// Censure récursivement une valeur JSON.
fn scrub_value(value: &mut sentry::protocol::Value) {
    use sentry::protocol::Value;
    match value {
        Value::String(text) => *text = redact(text),
        Value::Array(items) => items.iter_mut().for_each(scrub_value),
        Value::Object(fields) => fields.values_mut().for_each(scrub_value),
        _ => {}
    }
}

type BoxedLayer = Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>;

/// Écrivain qui censure chaque ligne avant de l'écrire.
///
/// Le fichier de journal est exactement ce qu'on demande à un joueur de joindre
/// quand quelque chose échoue — sur un salon Discord, dans un ticket. S'il
/// contient son jeton Microsoft, on a créé le problème qu'on voulait éviter.
/// La censure s'applique donc au fichier comme à Sentry, et pas seulement à ce
/// qui part sur le réseau.
struct RedactingWriter<W> {
    inner: W,
}

impl<W: std::io::Write> std::io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // `tracing` remet un événement formaté entier par appel : la censure
        // voit donc des lignes complètes, jamais un jeton coupé en deux.
        let text = String::from_utf8_lossy(buf);
        self.inner.write_all(redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

struct Redacting<M>(M);

impl<'a, M> tracing_subscriber::fmt::MakeWriter<'a> for Redacting<M>
where
    M: tracing_subscriber::fmt::MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.0.make_writer(),
        }
    }
}

/// Couche fichier, avec rotation quotidienne.
///
/// Un échec d'ouverture ne doit pas empêcher le programme de tourner : le
/// répertoire peut être en lecture seule ou plein. On perd le journal, pas
/// l'installation.
fn file_layer(
    component: &str,
) -> (
    Option<BoxedLayer>,
    Option<tracing_appender::non_blocking::WorkerGuard>,
    Option<PathBuf>,
) {
    let dir = log_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return (None, None, None);
    }
    purge_old_logs(&dir);

    let appender = tracing_appender::rolling::daily(&dir, format!("{component}.log"));
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(Redacting(writer))
        // Un fichier relu plus tard, souvent par quelqu'un d'autre : pas de
        // couleurs, et la cible du message est nécessaire pour situer.
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("debug,hyper=info,reqwest=info,rustls=info"))
        .boxed();

    (
        Some(layer),
        Some(guard),
        Some(dir.join(format!("{component}.log"))),
    )
}

/// Supprime les journaux trop anciens.
fn purge_old_logs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let limit = std::time::Duration::from_secs(KEEP_DAYS * 24 * 3600);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "log") || path.to_string_lossy().contains(".log") {
            let too_old = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| t.elapsed().map(|age| age > limit).unwrap_or(false))
                .unwrap_or(false);
            if too_old {
                std::fs::remove_file(&path).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_telemetrie_se_coupe() {
        // SAFETY : test mono-thread, variable restaurée aussitôt.
        unsafe {
            std::env::set_var("SAMFLIX_TELEMETRY", "0");
        }
        assert!(!telemetry_enabled());
        assert!(dsn().is_none());

        unsafe {
            std::env::set_var("SAMFLIX_TELEMETRY", "1");
        }
        assert!(telemetry_enabled());

        unsafe {
            std::env::remove_var("SAMFLIX_TELEMETRY");
        }
        assert!(telemetry_enabled());
    }

    #[test]
    fn un_dsn_vide_desactive_la_remontee() {
        unsafe {
            std::env::set_var("SENTRY_DSN", "   ");
        }
        assert!(dsn().is_none());
        unsafe {
            std::env::remove_var("SENTRY_DSN");
        }
    }

    #[test]
    fn un_attribut_de_journal_structure_est_censure() {
        use sentry::protocol::{LogAttribute, Value};

        // Les journaux structurés passent par `before_send_log`, pas par
        // `before_send` : ce filtre-là est le seul à les voir.
        let mut attribut = LogAttribute(Value::String(
            "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
        ));
        scrub_log_attribute(&mut attribut);
        match &attribut.0 {
            Value::String(text) => {
                assert!(!text.contains("eyJhbGci"));
                assert!(text.contains("[secret]"));
            }
            other => panic!("type inattendu : {other:?}"),
        }
    }

    #[test]
    fn un_attribut_numerique_reste_exploitable() {
        use sentry::protocol::{LogAttribute, Value};

        // Tailles, durées, codes HTTP : rien à censurer, et ils servent au tri.
        let mut attribut = LogAttribute(Value::from(2155935));
        scrub_log_attribute(&mut attribut);
        assert_eq!(attribut.0, Value::from(2155935));
    }

    #[test]
    fn le_fichier_de_journal_ne_recoit_pas_les_secrets() {
        use std::io::Write;

        // Le scénario redouté : un joueur joint son journal à un ticket. Ce qui
        // est écrit sur le disque doit déjà être censuré, pas seulement ce qui
        // part vers Sentry.
        let mut tampon = Vec::new();
        {
            let mut writer = RedactingWriter { inner: &mut tampon };
            writeln!(
                writer,
                "INFO échange abouti access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ"
            )
            .unwrap();
        }

        let ecrit = String::from_utf8(tampon).unwrap();
        assert!(
            !ecrit.contains("eyJhbGci"),
            "jeton écrit en clair : {ecrit}"
        );
        assert!(ecrit.contains("[secret]"));
        assert!(ecrit.contains("échange abouti"));
    }

    #[test]
    fn les_journaux_vivent_sous_le_repertoire_de_donnees() {
        assert!(log_dir().ends_with("logs"));
        assert!(log_dir().starts_with(mc_dl::data_dir()));
    }
}
