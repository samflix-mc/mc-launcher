//! Le code que l'utilisateur saisit pour autoriser le launcher.

/// Ce qu'on affiche à l'utilisateur pour qu'il autorise le launcher.
///
/// `verification_uri_directe` préremplit le code dans l'URL : un clic au lieu
/// d'un copier-coller. Les deux sont donnés, parce qu'un terminal qui n'ouvre
/// pas de navigateur a besoin de la forme longue.
#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_directe: String,
}
