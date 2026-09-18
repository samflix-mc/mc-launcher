use super::choose;
use crate::fixtures::environment;
use crate::manifest::Manifest;

const SERVERS: &str = r#"{"production":{"host":"mc.ggy.info"},
                           "development":{"host":"78.46.100.5","port":25566}}"#;

fn manifest(servers: &str) -> Manifest {
    let raw = format!(
        r#"{{"schema":1,"name":"samflix","minecraft":"1.21.1",
             "loader":{{"type":"neoforge","version":"21.1.250"}},
             "servers":{servers}}}"#
    );
    Manifest::parse(raw.as_bytes()).unwrap()
}

/// `--server` takes priority over what the pack declares, and the source is
/// retained: someone diagnosing an ejection needs to know which of the two
/// they're looking at.
#[test]
fn an_explicit_request_takes_priority_and_is_known() {
    let _env = environment("production");

    let (target, explicit, _) = choose(&manifest(SERVERS), Some("test.example.com:25577".into()));

    assert_eq!(target.as_deref(), Some("test.example.com:25577"));
    assert!(explicit);
}

/// Absent that, the one the pack declares for this binary's environment. The
/// manifest is the same everywhere: it's up to the client to choose, and it
/// chooses with what CI froze into it at build time.
#[test]
fn absent_that_the_pack_decides_by_environment() {
    let env = environment("production");

    let (target, explicit, environment) = choose(&manifest(SERVERS), None);
    // Without a declared port, the address sticks to the host: the game
    // defaults to 25565.
    assert_eq!(target.as_deref(), Some("mc.ggy.info"));
    assert!(!explicit);
    assert_eq!(environment, mc_log::Environment::Production);

    env.set("development");
    let (target, _, _) = choose(&manifest(SERVERS), None);
    assert_eq!(target.as_deref(), Some("78.46.100.5:25566"));
}

/// No declared server is not an error: preproduction has no Minecraft
/// servers behind it, and the game launches there without joining anything.
#[test]
fn no_declared_server_is_not_an_error() {
    let _env = environment("preproduction");

    let (target, explicit, _) = choose(&manifest(SERVERS), None);
    assert!(target.is_none(), "{target:?}");
    assert!(!explicit);
}
