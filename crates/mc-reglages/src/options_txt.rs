//! Fusionner nos clés dans le `options.txt` du jeu.
//!
//! ## Pourquoi fusionner et non écrire
//!
//! `options.txt` appartient à Minecraft. Il contient les touches, le volume,
//! la langue, et une ligne `version:` sur laquelle le jeu fonde ses correctifs
//! de données d'une version à l'autre. Le réécrire entièrement ferait perdre
//! tout cela.
//!
//! On ne touche donc qu'aux clés qu'on connaît, on garde les autres à leur
//! place et dans leur ordre, et l'on n'ajoute à la fin que ce qui manquait.
//!
//! ## Pourquoi on ne le CRÉE pas
//!
//! Un `options.txt` absent veut dire que le jeu n'a jamais démarré sur cette
//! instance. En fabriquer un partiel lui ferait perdre la ligne `version:`, et
//! il appliquerait ses correctifs de données comme si le fichier venait d'une
//! version inconnue. La page de configuration le dit au joueur — « ces
//! réglages s'appliqueront après votre première partie » — au lieu d'échouer
//! en silence.
//!
//! ## Pourquoi c'est une fonction pure
//!
//! Elle prend le texte et rend le texte. Tout le reste — lire, écrire, refuser
//! pendant une partie — appartient à l'appelant. C'est ce qui permet
//! d'éprouver les cas qui comptent : une clé déjà là, une clé absente, un
//! fichier avec des lignes qu'on ne connaît pas, un fichier vide.

use crate::types::{Fenetre, Jeu};

/// Les clés que nous pilotons, et rien d'autre.
///
/// `graphicsMode` en est délibérément absent : les shaders le remplacent, et
/// l'imposer depuis le launcher déferait ce qu'Iris a réglé.
fn nos_cles(jeu: &Jeu, fenetre: &Fenetre) -> Vec<(&'static str, String)> {
    vec![
        ("renderDistance", jeu.render_distance.to_string()),
        ("simulationDistance", jeu.simulation_distance.to_string()),
        (
            "maxFps",
            // Au-delà de 260, Minecraft n'attend pas un nombre mais le mot
            // « max ». Écrire 261 y serait relu comme une valeur invalide et
            // le jeu retomberait sur son défaut, sans rien dire.
            if jeu.max_fps >= 260 {
                "260".to_string()
            } else {
                jeu.max_fps.to_string()
            },
        ),
        ("guiScale", jeu.gui_scale.to_string()),
        ("enableVsync", jeu.vsync.to_string()),
        // Dérivée du mode : voir `Fenetre::plein_ecran`.
        ("fullscreen", fenetre.plein_ecran().to_string()),
    ]
}

/// Rend le contenu d'`options.txt` avec nos clés à jour.
///
/// Les clés inconnues sont conservées à leur place et dans leur ordre ; les
/// nôtres sont remplacées sur place quand elles existent, et ajoutées à la fin
/// sinon.
pub fn fusionner(existant: &str, jeu: &Jeu, fenetre: &Fenetre) -> String {
    let voulues = nos_cles(jeu, fenetre);
    let mut posees = vec![false; voulues.len()];

    let mut lignes: Vec<String> = Vec::new();
    for ligne in existant.lines() {
        // `options.txt` est du « clé:valeur », une paire par ligne. Une ligne
        // sans deux-points n'est pas une clé : on la garde telle quelle plutôt
        // que de la deviner.
        let cle = ligne.split_once(':').map(|(cle, _)| cle);
        match cle.and_then(|cle| voulues.iter().position(|(nom, _)| *nom == cle)) {
            Some(rang) => {
                lignes.push(format!("{}:{}", voulues[rang].0, voulues[rang].1));
                posees[rang] = true;
            }
            None => lignes.push(ligne.to_string()),
        }
    }

    for (rang, (nom, valeur)) in voulues.iter().enumerate() {
        if !posees[rang] {
            lignes.push(format!("{nom}:{valeur}"));
        }
    }

    let mut texte = lignes.join("\n");
    // Minecraft écrit toujours une ligne finale. Ne pas la remettre ferait que
    // le fichier change de forme à chaque aller-retour entre lui et nous.
    if !texte.is_empty() {
        texte.push('\n');
    }
    texte
}

#[cfg(test)]
#[path = "options_txt.test.rs"]
mod tests;
