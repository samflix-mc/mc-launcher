//! Mojang's vocabulary for naming a system, and its Maven paths.

pub fn mojang_os() -> &'static str {
    mojang_os_name(std::env::consts::OS)
}

pub fn mojang_arch() -> &'static str {
    mojang_arch_name(std::env::consts::ARCH)
}

/// The mapping, separated from reading `std::env::consts`.
///
/// These constants are fixed at compile time: a suite that only runs on
/// Linux would say nothing about the other two cases, and yet that's exactly
/// where the name of a file to download is decided. Taking them as an
/// argument is the only way to check the whole table from any machine.
fn mojang_os_name(os: &str) -> &'static str {
    match os {
        "macos" => "osx",
        "windows" => "windows",
        // Everything else passes for a Unix: that's what BSDs get, and
        // offering them the Linux libraries serves them better than nothing.
        _ => "linux",
    }
}

fn mojang_arch_name(arch: &str) -> &'static str {
    match arch {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// Maven path of a library whose `path` Mojang doesn't publish.
///
/// `group:artifact:version[:classifier]` becomes
/// `group/in/folders/artifact/version/artifact-version[-classifier].jar`.
/// Rare for vanilla libraries, systematic for the ones NeoForge adds.
pub fn maven_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?.replace('.', "/");
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();

    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}

#[cfg(test)]
#[path = "platform.test.rs"]
mod tests;
