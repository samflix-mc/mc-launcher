//! D'où vient la valeur : le lancement, la compilation, ou le défaut.

use super::Environment;

/// Valeur figée dans le binaire à la compilation, posée par la CI.
const COMPILED: Option<&str> = option_env!("SAMFLIX_ENV");

/// Environnement de cette exécution.
pub fn current() -> Environment {
    resolve(std::env::var("SAMFLIX_ENV").ok().as_deref(), COMPILED)
}

/// Où l'environnement a-t-il été décidé ? Affiché par le diagnostic, pour
/// qu'un environnement inattendu se remonte à sa source en une lecture.
pub fn origin() -> &'static str {
    match (
        std::env::var("SAMFLIX_ENV")
            .ok()
            .as_deref()
            .and_then(Environment::parse),
        COMPILED.and_then(Environment::parse),
    ) {
        (Some(_), _) => "variable SAMFLIX_ENV au lancement",
        (None, Some(_)) => "SAMFLIX_ENV figé à la compilation",
        (None, None) => "défaut, aucune déclaration",
    }
}

/// Résolution séparée de la lecture de l'environnement, pour être testable.
fn resolve(runtime: Option<&str>, compiled: Option<&str>) -> Environment {
    runtime
        .and_then(Environment::parse)
        .or_else(|| compiled.and_then(Environment::parse))
        .unwrap_or(Environment::Local)
}

#[cfg(test)]
#[path = "resolution.test.rs"]
mod tests;
