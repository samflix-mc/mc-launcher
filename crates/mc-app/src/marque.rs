//! Sous quel nom le launcher se présente.
//!
//! « samflix-mc » est le nom du réseau aujourd'hui, pas une constante du
//! produit. L'écrire dans le gabarit HTML et dans `tauri.conf.json` obligeait
//! à traverser les deux pour le changer, et à ne pas en oublier un.
//!
//! Il est donc lu **à la compilation**, dans `MC_LAUNCHER_NOM` :
//!
//! ```sh
//! MC_LAUNCHER_NOM="Mon Réseau" cargo tauri build
//! ```
//!
//! ## Pourquoi à la compilation et non au lancement
//!
//! Une variable d'environnement lue au démarrage ferait changer le nom selon
//! le shell d'où l'on clique — c'est-à-dire jamais chez un joueur, qui
//! double-clique sur une icône. Le nom appartient au binaire qu'on distribue,
//! au même titre que son icône : il se fixe quand on le fabrique.
//!
//! `build.rs` déclare la variable à Cargo, sans quoi la changer ne
//! déclencherait aucune recompilation et le binaire garderait l'ancien nom.

use serde::Serialize;

/// Ce que le launcher affiche de lui-même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Marque {
    /// Le nom complet, en titre de fenêtre et en en-tête.
    pub nom: String,
    /// Deux lettres pour le sceau rond.
    pub sceau: String,
}

/// Le nom retenu, figé à la compilation.
pub fn nom() -> &'static str {
    // `option_env!` et non `env!` : une compilation sans la variable doit
    // réussir, et retomber sur le nom d'aujourd'hui.
    match option_env!("MC_LAUNCHER_NOM") {
        Some(nom) if !nom.is_empty() => nom,
        _ => "samflix-mc",
    }
}

/// Les deux lettres du sceau.
///
/// Tirées du nom plutôt que réglées à part : un nom changé sans son sceau
/// donnerait un rond qui contredit l'en-tête juste à côté.
pub fn sceau(nom: &str) -> String {
    let mots: Vec<&str> = nom
        .split(|c: char| !c.is_alphanumeric())
        .filter(|mot| !mot.is_empty())
        .collect();

    let lettres: String = match mots.as_slice() {
        // Un seul mot : ses deux premières lettres. « samflix » → « SA ».
        [seul] => seul.chars().take(2).collect(),
        // Plusieurs : l'initiale des deux premiers. « Mon Réseau » → « MR ».
        [premier, second, ..] => premier
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
        [] => String::new(),
    };

    if lettres.is_empty() {
        // Un nom sans une seule lettre ni un seul chiffre reste possible ; un
        // sceau vide ferait un rond muet.
        return "??".to_string();
    }
    lettres.to_uppercase()
}

impl Marque {
    pub fn courante() -> Self {
        let nom = nom();
        Self {
            nom: nom.to_string(),
            sceau: sceau(nom),
        }
    }
}

#[cfg(test)]
#[path = "marque.test.rs"]
mod tests;
