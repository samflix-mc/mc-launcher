use super::{maven_path, mojang_arch, mojang_arch_name, mojang_os, mojang_os_name};

/// These names aren't ours: they're the keys under which Mojang publishes its
/// native libraries. "osx", not "macos"; "arm64", not "aarch64" — straying
/// from them downloads a file that doesn't exist.
#[test]
fn each_system_carries_the_name_mojang_publishes() {
    assert_eq!(mojang_os_name("macos"), "osx");
    assert_eq!(mojang_os_name("windows"), "windows");
    assert_eq!(mojang_os_name("linux"), "linux");
    // An unexpected system is treated as a Unix rather than rejected: that's
    // the case for BSDs.
    assert_eq!(mojang_os_name("freebsd"), "linux");
}

#[test]
fn each_architecture_carries_the_name_mojang_publishes() {
    assert_eq!(mojang_arch_name("x86"), "x86");
    assert_eq!(mojang_arch_name("aarch64"), "arm64");
    assert_eq!(mojang_arch_name("x86_64"), "x86_64");
    assert_eq!(mojang_arch_name("riscv64"), "x86_64");
}

/// The table is useless if what queries it doesn't give it the current
/// machine: these two are the only thing the rest of the crate calls.
#[test]
fn the_current_machine_is_named_by_the_same_table() {
    assert_eq!(mojang_os(), mojang_os_name(std::env::consts::OS));
    assert_eq!(mojang_arch(), mojang_arch_name(std::env::consts::ARCH));
}

#[test]
fn maven_path_with_and_without_classifier() {
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
    assert!(maven_path("incomplete").is_none());
}
