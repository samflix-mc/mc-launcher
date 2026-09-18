use super::{install_client, installer_url, version_id};
use crate::fixtures::Tree;

#[test]
fn produced_version_identifier() {
    assert_eq!(version_id("21.1.250"), "neoforge-21.1.250");
}

#[test]
fn installer_url_value() {
    assert_eq!(
        installer_url("21.1.250"),
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.250/neoforge-21.1.250-installer.jar"
    );
}

/// The installer applies binary patches: on a slow machine that's a minute
/// during which nothing moves. Rerunning it when the descriptor is already
/// there would make every launch pay for that minute — and that path is also
/// the only one that works offline.
#[tokio::test]
async fn an_already_done_installation_does_not_rerun_the_installer() {
    let tree = Tree::new("neoforge-already-there");
    tree.version("neoforge-21.1.250", r#"{"id":"neoforge-21.1.250"}"#);

    let produced = install_client(
        "21.1.250",
        &tree.shared(),
        &tree.root.join("cache"),
        // A Java that doesn't exist: if the installer were rerun, the call
        // would fail instead of returning the path.
        std::path::Path::new("/usr/lib/jvm/absent/bin/java"),
        &mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap(),
    )
    .await
    .expect("the descriptor is already there");

    assert_eq!(
        produced,
        tree.shared()
            .join("versions")
            .join("neoforge-21.1.250")
            .join("neoforge-21.1.250.json")
    );
}
