use super::maven_path;

#[test]
fn chemin_maven_avec_et_sans_classifier() {
    assert_eq!(
        maven_path("org.lwjgl:lwjgl:3.3.3").unwrap(),
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
    );
    assert_eq!(
        maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
    );
    assert_eq!(
        maven_path("net.neoforged:neoforge:21.1.250:client").unwrap(),
        "net/neoforged/neoforge/21.1.250/neoforge-21.1.250-client.jar"
    );
    assert!(maven_path("incomplet").is_none());
}
