//! Censure des secrets avant qu'un texte ne quitte la machine.
//!
//! Ce launcher manipule des jetons d'accès Microsoft, des jetons Xbox Live, un
//! jeton Minecraft et une clé d'API CurseForge. Un rapport d'incident est un
//! texte libre : message d'erreur, chemin de fichier, URL, trace d'appels. Rien
//! n'empêche un jeton de s'y retrouver — une URL signée, un `Debug` de
//! structure, un message d'API qui répète la requête.
//!
//! Le filtre s'applique à **tout** ce qui part vers Sentry, événements comme
//! fils d'Ariane, et au fichier de journal. Il ne cherche pas à être malin : il
//! repère un petit nombre de formes connues et les remplace entièrement. Rendre
//! un incident un peu moins lisible est sans commune mesure avec la publication
//! d'un jeton qui donne accès à un compte Microsoft.
mod curseur;
mod home;
mod jwt;
mod mot_cle;
mod tables;

/// Ce qui remplace un secret.
pub(crate) const MASK: &str = "[secret]";

/// Remplace les secrets d'un texte.
///
/// Trois passes qui ne se recouvrent pas : la forme d'abord — un JWT se
/// reconnaît seul —, puis ce qu'un mot-clé annonce, puis le chemin personnel.
pub fn redact(text: &str) -> String {
    let text = jwt::redact_jwt(text);
    let text = mot_cle::redact_after_keywords(&text);
    home::redact_home(&text)
}

#[cfg(test)]
#[path = "redact.test.rs"]
mod tests;
