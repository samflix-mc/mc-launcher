//! Le vocabulaire de Mojang pour désigner un système, et ses chemins Maven.

pub fn mojang_os() -> &'static str {
    nom_mojang_du_systeme(std::env::consts::OS)
}

pub fn mojang_arch() -> &'static str {
    nom_mojang_de_l_architecture(std::env::consts::ARCH)
}

/// La correspondance, séparée de la lecture de `std::env::consts`.
///
/// Ces constantes sont figées à la compilation : une suite qui ne tourne que
/// sur Linux ne dirait rien des deux autres cas, et c'est pourtant là que se
/// joue le nom d'un fichier à télécharger. Les prendre en argument est la
/// seule façon de vérifier la table entière depuis n'importe quel poste.
fn nom_mojang_du_systeme(os: &str) -> &'static str {
    match os {
        "macos" => "osx",
        "windows" => "windows",
        // Tout le reste passe pour un Unix : c'est ce que reçoivent les BSD,
        // et proposer les bibliothèques Linux leur vaut mieux que rien.
        _ => "linux",
    }
}

fn nom_mojang_de_l_architecture(arch: &str) -> &'static str {
    match arch {
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
