//! Le vocabulaire de Mojang pour désigner un système, et ses chemins Maven.

pub fn mojang_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

pub fn mojang_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// Chemin Maven d'une bibliothèque dont Mojang ne publie pas le `path`.
///
/// `groupe:artefact:version[:classifier]` devient
/// `groupe/en/dossiers/artefact/version/artefact-version[-classifier].jar`.
/// Rare pour les bibliothèques vanilla, systématique pour celles que NeoForge
/// ajoute.
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
#[path = "plateforme.test.rs"]
mod tests;
