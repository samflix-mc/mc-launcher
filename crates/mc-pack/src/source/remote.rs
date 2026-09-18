//! The three published addresses, and the environment that picks one.

mod fetch;

pub(super) use fetch::load_remote;

pub const URL_PRODUCTION: &str = "https://mc-launcher.ggy.info/pack/samflix.json";
pub const URL_PREPRODUCTION: &str = "https://mc-launcher-staging.ggy.info/pack/samflix.json";
pub const URL_DEVELOPMENT: &str = "https://mc-launcher-dev.ggy.info/pack/samflix.json";

/// The pack for this binary's environment.
///
/// `mc-log` already knows where this binary comes from. That knowledge went
/// unused: the pack's address was hardcoded to production, so a preproduction
/// build would download the production pack and therefore exercise none of
/// what it was supposed to exercise.
///
/// The environment is **declared**, and in this order: `SAMFLIX_ENV` at
/// launch first, then `SAMFLIX_ENV` frozen at compile time by CI, then
/// `local`. The launch-time variable therefore wins over everything — that's
/// what lets a production binary be replayed against the dev pack without
/// recompiling, and that's also why a shell where it lingers changes the
/// target without warning.
///
/// A hand-compiled binary targets **dev**, and that's the cheaper of the two
/// mistakes: nobody compiles this launcher to play, and a dev pack installed
/// by mistake is fixed with a `--source`. The reverse — a working binary that
/// installs the players' pack — is harder to notice.
///
/// `--source` stays the top priority over everything, and that's what lets
/// any binary exercise any environment.
pub fn default_url() -> &'static str {
    use mc_log::environment::Environment;
    match mc_log::environment::current() {
        Environment::Production => URL_PRODUCTION,
        Environment::Preproduction => URL_PREPRODUCTION,
        Environment::Development | Environment::Local => URL_DEVELOPMENT,
    }
}
