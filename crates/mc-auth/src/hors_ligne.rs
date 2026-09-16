//! Une session sans Microsoft, pour développer et pour les serveurs
//! `online-mode=false`.

use crate::{Profile, Session};

/// Profil hors-ligne, pour développer sans jeton Microsoft.
///
/// Tant que l'inscription d'application n'est pas approuvée,
/// `login_with_xbox` répond 403 et aucune session réelle n'est possible. Le
/// reste du launcher — installation des mods, ligne de commande JVM, Quick
/// Play, interface — n'a pourtant besoin que d'un pseudo et d'un UUID.
///
/// L'UUID suit **exactement** la règle du serveur vanilla :
/// `UUID.nameUUIDFromBytes("OfflinePlayer:<pseudo>")`, soit un UUID de
/// version 3 fondé sur MD5. C'est indispensable : les backends du réseau
/// tournent en `online-mode=false`, ils calculent l'UUID de cette façon, et un
/// UUID inventé au hasard donnerait un joueur différent à chaque connexion —
/// inventaire, position et permissions perdus.
///
/// Ne produit aucun jeton : la session renvoyée ne permet pas de rejoindre un
/// serveur en ligne, seulement un serveur en mode hors-ligne.
pub fn offline_session(name: &str) -> Session {
    use md5::{Digest, Md5};

    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // variante RFC 4122

    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();

    Session {
        minecraft_token: String::new(),
        refresh_token: None,
        profile: Profile {
            id: hex,
            name: name.to_owned(),
        },
    }
}

#[cfg(test)]
#[path = "hors_ligne.test.rs"]
mod tests;
