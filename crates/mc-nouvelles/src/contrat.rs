//! Ce que l'hôte publie, et ce que le launcher en fait.
//!
//! ## Le contrat, écrit ici et nulle part ailleurs
//!
//! ```jsonc
//! {
//!   "schema": 1,
//!   "billets": [
//!     {
//!       "id": "2026-09-saison-3",       // stable, sert de clé au front
//!       "titre": "La saison 3 ouvre",
//!       "date": "2026-09-18T18:00:00Z", // RFC 3339, en UTC
//!       "epinglee": true,               // facultatif, défaut false
//!       "image": "saison3.webp",        // facultatif, relatif au fil
//!       "corps": "Texte **markdown**."
//!     }
//!   ]
//! }
//! ```
//!
//! Un billet fautif est écarté SEUL : une date illisible sur un billet ne doit
//! pas faire disparaître les neuf autres. C'est la différence entre « la page
//! des news a un trou » et « la page des news est vide », et la seconde se lit
//! comme une panne du launcher.

use serde::{Deserialize, Serialize};

/// Version du format servi par l'hôte.
pub const SCHEMA: u32 = 1;

/// Ce que l'hôte sert, tel quel.
#[derive(Debug, Clone, Deserialize)]
pub struct FilBrut {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub billets: Vec<BilletBrut>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BilletBrut {
    pub id: String,
    pub titre: String,
    /// RFC 3339. Analysée ici plutôt que par le front : une date invalide doit
    /// écarter le billet, et le front n'a pas les moyens de le décider.
    pub date: String,
    #[serde(default)]
    pub epinglee: bool,
    #[serde(default)]
    pub image: Option<String>,
    pub corps: String,
}

/// Un billet retenu, son corps déjà analysé.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Billet {
    pub id: String,
    pub titre: String,
    pub date: String,
    pub epinglee: bool,
    /// L'URL absolue de l'image, déjà contrôlée. `None` s'il n'y en a pas ou
    /// si celle qu'on annonçait sortait de l'hôte du fil.
    pub image: Option<String>,
    /// Le corps, en arbre typé. **Jamais de HTML** : c'est la condition à
    /// laquelle le CSP a été desserré.
    pub corps: Vec<crate::arbre::Bloc>,
}

/// Le fil, prêt pour la fenêtre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fil {
    pub billets: Vec<Billet>,
    /// Le fil vient du cache et non du réseau.
    pub hors_ligne: bool,
    /// Combien de billets ont été écartés, et pourquoi — pour le journal, et
    /// pour qu'un fil à moitié fautif se remarque.
    pub ecartes: Vec<String>,
}

/// Trie et valide, en Rust.
///
/// ## Pourquoi le tri est ici
///
/// La même raison que pour la règle du bouton : une règle qui vit dans un
/// `computed()` d'Angular ne se vérifie qu'en vitest, et sort du périmètre de
/// mutation. Celle-ci — « les épinglés d'abord, puis du plus récent au plus
/// ancien » — est exactement le genre de règle dont l'inversion ne casse aucun
/// test d'affichage et se voit seulement à l'œil, des semaines plus tard.
///
/// Le tri porte sur la date RFC 3339 telle quelle : son format est
/// lexicographiquement ordonné, à condition d'être en UTC avec le même nombre
/// de chiffres — ce que `valider_la_date` impose.
pub fn ordonner(mut billets: Vec<Billet>) -> Vec<Billet> {
    billets.sort_by(|a, b| {
        b.epinglee
            .cmp(&a.epinglee)
            .then_with(|| b.date.cmp(&a.date))
            // À date et épinglage égaux, l'identifiant tranche : sans lui,
            // l'ordre dépendrait de celui du JSON reçu, et deux chargements du
            // même fil pourraient ne pas donner le même écran.
            .then_with(|| a.id.cmp(&b.id))
    });
    billets
}

/// Une date est-elle du RFC 3339 en UTC, tel qu'on l'exige ?
///
/// Contrôle volontairement STRICT sur la forme, et non une analyse complète :
/// on ne veut pas d'une bibliothèque de dates pour valider ce que l'hôte
/// publie, et la forme suffit à garantir ce dont le tri a besoin — une chaîne
/// dont l'ordre lexicographique est l'ordre chronologique.
///
/// Un décalage horaire (« +02:00 ») est donc REFUSÉ : il casserait cette
/// propriété, et rien dans l'affichage ne le montrerait.
pub fn date_valide(date: &str) -> bool {
    let octets = date.as_bytes();
    if octets.len() != 20 {
        return false;
    }
    let chiffres = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    if !chiffres.iter().all(|&i| octets[i].is_ascii_digit()) {
        return false;
    }
    octets[4] == b'-'
        && octets[7] == b'-'
        && octets[10] == b'T'
        && octets[13] == b':'
        && octets[16] == b':'
        && octets[19] == b'Z'
}

#[cfg(test)]
#[path = "contrat.test.rs"]
mod tests;
