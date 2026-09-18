//! Le corps d'un billet, en arbre typé.
//!
//! ## Pourquoi un arbre et pas du HTML
//!
//! Le CSP du launcher desserre `style-src` jusqu'à `'unsafe-inline'`. Ce
//! desserrage ne tient qu'à une condition : aucun balisage ne vient d'ailleurs
//! que du compilateur Angular. Rendre du HTML produit par un hôte distant —
//! fût-il le nôtre — dans une origine où `invoke` est joignable annulerait
//! cette condition d'un coup.
//!
//! Le markdown est donc analysé ICI, en Rust, vers des variantes fermées. Le
//! front les parcourt avec un `@switch` et ne voit jamais une chaîne qu'il
//! devrait interpréter. `pnpm invariants` vérifie qu'aucun `innerHTML` ne
//! traîne côté front ; ce module est l'autre moitié de la garantie.
//!
//! ## Pourquoi `#[serde(tag = "type")]`
//!
//! Sans lui, serde produit la représentation « externally tagged » —
//! `{"Paragraphe": {...}}` — et le front devrait inspecter la première CLÉ
//! d'un objet pour savoir à quoi il a affaire. Avec, il lit un champ `type` et
//! écrit un `@switch` ordinaire.

use serde::Serialize;

/// Un bloc de niveau supérieur.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Bloc {
    Paragraphe {
        contenu: Vec<Inline>,
    },
    /// Un titre. Le niveau est borné à 2..=4 : le titre du billet occupe déjà
    /// le niveau 1 de la page, et un `#` du corps ne doit pas le concurrencer.
    Titre {
        niveau: u8,
        contenu: Vec<Inline>,
    },
    Liste {
        puces: Vec<Vec<Inline>>,
    },
    /// Une ligne de séparation.
    Separateur,
}

/// Un fragment de texte.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Texte {
        texte: String,
    },
    Gras {
        texte: String,
    },
    Italique {
        texte: String,
    },
    Code {
        texte: String,
    },
    /// Un lien. L'URL est déjà filtrée — voir `liens::acceptable`.
    ///
    /// Elle ne sera JAMAIS suivie dans la fenêtre : le front la confie à
    /// `ouvrirPage()`, qui la donne au navigateur du système. Le greffon de
    /// navigation refuserait de toute façon.
    Lien {
        texte: String,
        href: String,
    },
}
