//! Provoque une panique pour vérifier qu'elle devient un incident.
//!
//!     cargo run -p mc-log --example panique
//!     SAMFLIX_TELEMETRY=0 cargo run -p mc-log --example panique   # sans envoi
//!
//! Complète `mc-pack diagnostic --incident-test`, qui ne valide que le
//! transport : ici c'est le gestionnaire de panique qui est éprouvé, de la
//! panique jusqu'à l'envoi. Le message contient volontairement un faux jeton,
//! pour qu'on puisse vérifier dans Sentry qu'il en est ressorti censuré.

fn main() {
    let _log = mc_log::init("mc-log-panique");

    tracing::info!("avant la panique — ce message doit devenir un fil d'Ariane");
    tracing::warn!("un avertissement, pour vérifier qu'il accompagne l'incident");

    panic!(
        "panique volontaire de vérification, avec access_token=\
         eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA qui ne doit pas sortir"
    );
}
