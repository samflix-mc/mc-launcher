//! D'où vient ce binaire, et donc où classer ses incidents.
//!
//! Un environnement décrit un **déploiement**, pas un profil de compilation.
//! La confusion des deux a une conséquence immédiate : un `cargo run --release`
//! lancé sur un poste de développement n'a pas `debug_assertions`, et se
//! retrouve classé en production. Les premiers incidents de test du projet y
//! sont arrivés de cette façon, dans un environnement où rien n'avait jamais
//! été déployé.
//!
//! L'environnement est donc **déclaré**, jamais déduit :
//!
//! 1. `SAMFLIX_ENV` au lancement — pour qu'un préprod puisse être rejoué en
//!    local sans recompiler ;
//! 2. `SAMFLIX_ENV` à la compilation — c'est la CI qui le pose, selon ce
//!    qu'elle construit ;
//! 3. à défaut, `local` — un binaire compilé à la main est local, qu'il soit en
//!    debug ou en release.
//!
//! Le défaut compte plus qu'il n'y paraît : il vaut mieux qu'un incident de
//! production passe pour local que l'inverse. Le premier se remarque parce
//! qu'on cherche l'incident et qu'on ne le trouve pas ; le second pollue
//! silencieusement le seul environnement qu'on surveille vraiment.

/// Environnement de déploiement, au sens de Sentry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Compilé et lancé sur le poste de quelqu'un.
    Local,
    /// Construit par la CI sur une branche de travail.
    Development,
    /// Construit par la CI sur la branche principale, avant publication.
    Preproduction,
    /// Binaire publié sur un tag.
    Production,
}

impl Environment {
    /// Nom envoyé à Sentry.
    pub fn as_str(self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Development => "development",
            Environment::Preproduction => "preproduction",
            Environment::Production => "production",
        }
    }

    /// Lit un nom d'environnement, alias courants compris.
    ///
    /// `dev`, `preprod` et `staging` sont ce qu'on tape et ce que les CI
    /// emploient ; les refuser ferait silencieusement retomber sur le défaut,
    /// c'est-à-dire exactement l'erreur qu'on cherche à éviter.
    pub fn parse(text: &str) -> Option<Environment> {
        match text.trim().to_ascii_lowercase().as_str() {
            "local" | "dev-local" => Some(Environment::Local),
            "development" | "dev" => Some(Environment::Development),
            "preproduction" | "preprod" | "pre-prod" | "staging" => {
                Some(Environment::Preproduction)
            }
            "production" | "prod" => Some(Environment::Production),
            _ => None,
        }
    }

    /// Cet environnement correspond-il à un binaire distribué ?
    ///
    /// Sert à décider de ce qui est acceptable par défaut : on est plus
    /// bavard sur un poste de développement que chez un joueur.
    pub fn is_deployed(self) -> bool {
        matches!(self, Environment::Preproduction | Environment::Production)
    }
}

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
#[path = "environment.test.rs"]
mod tests;
