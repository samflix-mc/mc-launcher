use super::usage;
use crate::commands::fixtures::environment;

/// Help names the default source, and that depends on the binary's
/// environment: a preproduction build announcing the production pack would
/// send someone to the wrong place.
///
/// Help goes to standard error; what the test checks is that it assembles
/// without panicking, and that the address announced follows the
/// environment.
#[test]
fn help_announces_the_source_of_this_environment() {
    let env = environment("production");
    usage();
    assert_eq!(
        mc_pack::source::default_url(),
        mc_pack::source::URL_PRODUCTION
    );

    env.set("preproduction");
    usage();
    assert_eq!(
        mc_pack::source::default_url(),
        mc_pack::source::URL_PREPRODUCTION
    );

    // A binary compiled by hand targets dev: a dev pack installed by
    // mistake is fixed with a --source, the reverse is harder to notice.
    for local in ["development", "local"] {
        env.set(local);
        assert_eq!(
            mc_pack::source::default_url(),
            mc_pack::source::URL_DEVELOPMENT,
            "for \"{local}\""
        );
    }
}
