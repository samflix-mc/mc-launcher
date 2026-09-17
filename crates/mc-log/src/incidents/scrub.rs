//! Censure de tout ce qui sort vers Sentry.

use crate::redact::redact;

/// Censure un événement de bout en bout.
pub(crate) fn scrub_event(event: &mut sentry::protocol::Event<'static>) {
    if let Some(message) = event.message.take() {
        event.message = Some(redact(&message));
    }
    for exception in &mut event.exception.values {
        exception.value = exception.value.as_deref().map(redact);
        scrub_stacktrace(exception.stacktrace.as_mut());
        scrub_stacktrace(exception.raw_stacktrace.as_mut());
    }
    // `attach_stacktrace` fait joindre la pile du fil courant par une
    // intégration du SDK, et les intégrations tournent avant `before_send`. Un
    // événement sans exception — un `capture_message`, un plantage du jeu —
    // n'expose donc ses chemins de compilation que par là, à côté de la boucle
    // qui les censure.
    for thread in &mut event.threads.values {
        scrub_stacktrace(thread.stacktrace.as_mut());
        scrub_stacktrace(thread.raw_stacktrace.as_mut());
    }
    scrub_stacktrace(event.stacktrace.as_mut());
    for value in event.extra.values_mut() {
        scrub_value(value);
    }
    // Les champs d'un `tracing::error!` n'arrivent pas dans `extra` :
    // `sentry-tracing` les range dans le contexte « Rust Tracing Fields ».
    // Sans ce passage, un `erreur = ?error` partirait tel quel — soit le canal
    // le plus riche de tous, et le seul que la censure aurait laissé filer.
    for context in event.contexts.values_mut() {
        if let sentry::protocol::Context::Other(fields) = context {
            for value in fields.values_mut() {
                scrub_value(value);
            }
        }
    }
    for tag in event.tags.values_mut() {
        *tag = redact(tag);
    }
}

/// Censure les chemins d'une pile d'appels.
///
/// Un chemin absolu porte le nom de compte de celui qui a compilé, et une
/// variable capturée porte ce qu'elle porte.
fn scrub_stacktrace(stacktrace: Option<&mut sentry::protocol::Stacktrace>) {
    let Some(stacktrace) = stacktrace else {
        return;
    };
    for frame in &mut stacktrace.frames {
        frame.filename = frame.filename.as_deref().map(redact);
        frame.abs_path = frame.abs_path.as_deref().map(redact);
        for value in frame.vars.values_mut() {
            scrub_value(value);
        }
    }
}

/// Censure un attribut de journal structuré.
///
/// Les champs d'un événement `tracing` deviennent des attributs : un
/// `tracing::info!(url = %url, ...)` les expose tels quels. Seules les chaînes
/// peuvent porter un secret ; les nombres et booléens sont laissés intacts,
/// puisqu'ils restent utiles au tri et au filtrage.
pub(crate) fn scrub_log_attribute(attribute: &mut sentry::protocol::LogAttribute) {
    use sentry::protocol::Value;
    if let Value::String(text) = &attribute.0 {
        attribute.0 = Value::String(redact(text));
    }
}

/// Censure récursivement une valeur JSON.
pub(crate) fn scrub_value(value: &mut sentry::protocol::Value) {
    use sentry::protocol::Value;
    match value {
        Value::String(text) => *text = redact(text),
        Value::Array(items) => items.iter_mut().for_each(scrub_value),
        Value::Object(fields) => fields.values_mut().for_each(scrub_value),
        _ => {}
    }
}

#[cfg(test)]
#[path = "scrub.test.rs"]
mod tests;
