//! Les trois adresses publiées, et l'environnement qui en choisit une.

mod recuperation;

pub(super) use recuperation::load_remote;

pub const URL_PRODUCTION: &str = "https://mc-launcher.ggy.info/pack/samflix.json";
pub const URL_PREPRODUCTION: &str = "https://mc-launcher-staging.ggy.info/pack/samflix.json";
pub const URL_DEVELOPPEMENT: &str = "https://mc-launcher-dev.ggy.info/pack/samflix.json";

/// Le pack de l'environnement de ce binaire.
///
/// `mc-log` sait déjà d'où vient ce binaire. Cette connaissance existait sans
/// servir : l'adresse du pack était écrite en dur sur la production, si bien
/// qu'une préproduction téléchargeait le pack de la production et n'éprouvait
/// donc rien de ce qu'elle était censée éprouver.
///
/// L'environnement est **déclaré**, et dans cet ordre : `SAMFLIX_ENV` au
/// lancement d'abord, puis `SAMFLIX_ENV` figé à la compilation par la CI, puis
/// `local`. La variable au lancement prime donc sur tout — c'est ce qui permet
/// de rejouer un binaire de production contre le pack de dev sans recompiler,
/// et ce qui explique qu'un shell où elle traîne change la cible sans prévenir.
///
/// Un binaire compilé à la main vise la **dev**, et c'est le choix le moins
/// coûteux des deux : personne ne compile ce launcher pour jouer, et un pack de
/// dev installé par erreur se corrige d'un `--source`. L'inverse — un binaire de
/// travail qui installe le pack des joueurs — est plus difficile à remarquer.
///
/// `--source` reste prioritaire sur tout, et c'est ce qui permet d'éprouver un
/// environnement depuis n'importe quel binaire.
pub fn url_par_defaut() -> &'static str {
    use mc_log::environment::Environment;
    match mc_log::environment::current() {
        Environment::Production => URL_PRODUCTION,
        Environment::Preproduction => URL_PREPRODUCTION,
        Environment::Development | Environment::Local => URL_DEVELOPPEMENT,
    }
}
