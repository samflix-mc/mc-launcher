//! Ce qui annonce un secret, et ce qui annonce seulement son annonce.

/// Mots qui annoncent une valeur sensible juste après.
pub(super) const KEYWORDS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "device_code",
    "minecraft_token",
    "api_key",
    "apikey",
    "api-key",
    "x-api-key",
    "x-api-token",
    "authorization",
    "password",
    "secret",
    "token",
];

/// Schémas d'authentification HTTP : le mot annonce la valeur, il n'est pas la
/// valeur.
///
/// Sans cette liste, seul « Bearer » était reconnu. Les autres schémas étaient
/// pris pour le secret lui-même : c'est le nom du schéma qui se faisait masquer,
/// et l'identifiant qui le suit partait en clair.
///
/// Un schéma n'est reconnu que s'il forme un mot à part *et* s'il introduit
/// réellement quelque chose ; sinon c'est lui, la valeur. Sans ces deux
/// conditions, « token=basicXXXX » laissait passer « basic » en clair, et
/// « password: digest » ne masquait plus rien du tout.
pub(super) const SCHEMES: &[&str] = &["bearer", "basic", "digest", "negotiate", "token", "dpop"];
