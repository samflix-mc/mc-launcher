//! Du markdown vers l'arbre typé.
//!
//! ## Un sous-ensemble, délibérément
//!
//! Paragraphes, titres, listes à puces, séparateurs ; gras, italique, code,
//! liens. Rien d'autre — ni tableaux, ni HTML brut, ni images dans le corps,
//! ni blocs de code multilignes.
//!
//! Ce n'est pas de la paresse : chaque construction reconnue est une forme de
//! plus que le front doit savoir rendre, et une occasion de plus qu'un contenu
//! distant a de produire quelque chose d'inattendu. La règle est que ce qu'on
//! ne reconnaît pas devienne du TEXTE, jamais du balisage — un `<script>` écrit
//! dans un billet ressort donc comme les onze caractères qu'il est.
//!
//! ## Pourquoi pas une bibliothèque
//!
//! Les analyseurs markdown complets produisent du HTML. Ceux qui produisent un
//! arbre tirent une dépendance large pour un sous-ensemble qu'on n'emploie pas,
//! et dont chaque extension future élargirait la surface sans qu'on le décide.
//! Deux cents lignes ici se relisent en entier.

use crate::arbre::{Bloc, Inline};

/// Analyse un corps de billet.
pub fn analyser(markdown: &str, hote_du_fil: &str) -> Vec<Bloc> {
    let mut blocs = Vec::new();
    let mut paragraphe: Vec<String> = Vec::new();
    let mut puces: Vec<Vec<Inline>> = Vec::new();

    // Referme ce qui était en cours avant d'ouvrir autre chose.
    macro_rules! fermer {
        ($blocs:expr, $paragraphe:expr, $puces:expr) => {
            if !$paragraphe.is_empty() {
                $blocs.push(Bloc::Paragraphe {
                    contenu: inlines(&$paragraphe.join(" "), hote_du_fil),
                });
                $paragraphe.clear();
            }
            if !$puces.is_empty() {
                $blocs.push(Bloc::Liste {
                    puces: std::mem::take(&mut $puces),
                });
            }
        };
    }

    for ligne in markdown.lines() {
        let nu = ligne.trim();

        if nu.is_empty() {
            fermer!(blocs, paragraphe, puces);
            continue;
        }

        // Un séparateur : trois tirets ou plus, et rien d'autre.
        if nu.len() >= 3 && nu.chars().all(|c| c == '-') {
            fermer!(blocs, paragraphe, puces);
            blocs.push(Bloc::Separateur);
            continue;
        }

        if let Some((niveau, texte)) = titre(nu) {
            fermer!(blocs, paragraphe, puces);
            blocs.push(Bloc::Titre {
                niveau,
                contenu: inlines(texte, hote_du_fil),
            });
            continue;
        }

        if let Some(puce) = nu.strip_prefix("- ").or_else(|| nu.strip_prefix("* ")) {
            // Une puce ferme le paragraphe mais PAS la liste : deux puces de
            // suite appartiennent à la même.
            if !paragraphe.is_empty() {
                blocs.push(Bloc::Paragraphe {
                    contenu: inlines(&paragraphe.join(" "), hote_du_fil),
                });
                paragraphe.clear();
            }
            puces.push(inlines(puce.trim(), hote_du_fil));
            continue;
        }

        // Une ligne ordinaire après une liste la ferme.
        if !puces.is_empty() {
            blocs.push(Bloc::Liste {
                puces: std::mem::take(&mut puces),
            });
        }
        paragraphe.push(nu.to_string());
    }

    fermer!(blocs, paragraphe, puces);
    blocs
}

/// Un titre, et son niveau borné.
///
/// Le niveau est ramené dans 2..=4 : le titre du billet occupe le niveau 1 de
/// la page, et un `#` du corps ne doit pas le concurrencer — ni dans le rendu,
/// ni pour un lecteur d'écran, qui se sert de la hiérarchie pour naviguer.
fn titre(ligne: &str) -> Option<(u8, &str)> {
    let diese = ligne.chars().take_while(|&c| c == '#').count();
    if diese == 0 || diese > 6 {
        return None;
    }
    let reste = ligne[diese..].strip_prefix(' ')?;
    let niveau = (diese as u8 + 1).clamp(2, 4);
    Some((niveau, reste.trim()))
}

/// Analyse le contenu d'une ligne.
fn inlines(texte: &str, hote_du_fil: &str) -> Vec<Inline> {
    let mut sortie = Vec::new();
    let mut tampon = String::new();
    let octets: Vec<char> = texte.chars().collect();
    let mut i = 0;

    /// Vide le tampon dans la sortie, s'il y a quelque chose.
    macro_rules! vider {
        ($sortie:expr, $tampon:expr) => {
            if !$tampon.is_empty() {
                $sortie.push(Inline::Texte {
                    texte: std::mem::take(&mut $tampon),
                });
            }
        };
    }

    while i < octets.len() {
        // Un lien : [texte](url)
        if octets[i] == '['
            && let Some((libelle, href, saut)) = lien(&octets[i..])
        {
            {
                match crate::liens::acceptable(&href, hote_du_fil) {
                    Some(sure) => {
                        vider!(sortie, tampon);
                        sortie.push(Inline::Lien {
                            texte: libelle,
                            href: sure,
                        });
                    }
                    // Une URL refusée ne fait pas disparaître le texte : le
                    // libellé devient du texte ordinaire. Effacer la phrase
                    // parce que son lien déplaît serait pire que de la garder
                    // sans lien.
                    None => {
                        tracing::warn!(href, "lien écarté d'un billet");
                        tampon.push_str(&libelle);
                    }
                }
                // `.max(1)`, pour la MÊME raison que plus bas : la boucle
                // doit progresser à chaque tour, et un `saut` nul la ferait
                // tourner sans fin sur le fil d'interface. `lien` ne rend
                // jamais zéro aujourd'hui — il faut au moins `[]()` — mais
                // c'est une propriété de `lien`, pas de cette boucle, et une
                // boucle ne doit pas dépendre pour sa terminaison d'une
                // fonction qu'on peut modifier ailleurs.
                i += saut.max(1);
                continue;
            }
        }

        // Les délimiteurs de mise en forme, du plus long au plus court. Ce qui
        // n'en est pas un est du texte, caractère par caractère : c'est la
        // règle qui fait qu'un `<script>` écrit dans un billet ressort comme
        // les onze caractères qu'il est.
        //
        // `avance.max(1)` : la boucle DOIT progresser à chaque tour.
        //
        // Ce n'est pas une précaution théorique. Le corps d'un billet vient
        // d'un hôte, il est analysé dans le fil d'interface, et une boucle qui
        // n'avance pas gèlerait la fenêtre entière — sans message, sans
        // plantage, et sans autre issue que de tuer le processus. Un analyseur
        // qui ne peut pas garantir sa progression n'a rien à faire sur ce
        // chemin-là.
        //
        // Aucune entrée réelle ne produit zéro aujourd'hui ; c'est exactement
        // pour cela qu'on l'écrit, plutôt que de découvrir le contraire chez
        // un joueur.
        match consomme(&octets, i, &mut sortie, &mut tampon) {
            Some(avance) => i += avance.max(1),
            None => {
                tampon.push(octets[i]);
                i += 1;
            }
        }
    }

    vider!(sortie, tampon);
    sortie
}

type Fabrique = fn(String) -> Inline;

/// Les délimiteurs reconnus, DU PLUS LONG AU PLUS COURT.
///
/// L'ordre n'est pas décoratif : `**gras**` commence par `*`, et tester
/// l'italique d'abord produirait un italique vide suivi du mot puis d'un
/// italique vide.
const DELIMITEURS: [(&str, Fabrique); 3] = [
    ("**", |texte| Inline::Gras { texte }),
    ("`", |texte| Inline::Code { texte }),
    ("*", |texte| Inline::Italique { texte }),
];

/// Tente de consommer un délimiteur à la position `i`.
fn consomme(
    octets: &[char],
    i: usize,
    sortie: &mut Vec<Inline>,
    tampon: &mut String,
) -> Option<usize> {
    for (marque, fabrique) in DELIMITEURS {
        let marque_chars: Vec<char> = marque.chars().collect();
        if !octets[i..].starts_with(&marque_chars[..]) {
            continue;
        }
        let (interieur, saut) = jusqu_a(&octets[i + marque_chars.len()..], marque)?;
        // Un délimiteur vide — `****` — n'est pas une mise en forme : on le
        // laisse devenir du texte plutôt que de produire un nœud vide que le
        // front devrait savoir ne pas rendre.
        if interieur.is_empty() {
            continue;
        }
        if !tampon.is_empty() {
            sortie.push(Inline::Texte {
                texte: std::mem::take(tampon),
            });
        }
        sortie.push(fabrique(interieur));
        return Some(marque_chars.len() + saut);
    }
    None
}

/// Le texte jusqu'à la prochaine occurrence de `marque`, et de combien avancer.
///
/// Rend `None` si la marque n'est jamais refermée : un `*` isolé au milieu
/// d'une phrase est un astérisque, pas une italique ouverte jusqu'à la fin du
/// billet.
fn jusqu_a(reste: &[char], marque: &str) -> Option<(String, usize)> {
    let marque_chars: Vec<char> = marque.chars().collect();
    let mut i = 0;
    while i + marque_chars.len() <= reste.len() {
        if reste[i..].starts_with(&marque_chars[..]) {
            return Some((reste[..i].iter().collect(), i + marque_chars.len()));
        }
        i += 1;
    }
    None
}

/// Un `[libellé](url)` complet, ou rien.
fn lien(reste: &[char]) -> Option<(String, String, usize)> {
    let ferme = reste.iter().position(|&c| c == ']')?;
    if reste.get(ferme + 1) != Some(&'(') {
        return None;
    }
    let fin = reste[ferme + 2..].iter().position(|&c| c == ')')? + ferme + 2;
    let libelle: String = reste[1..ferme].iter().collect();
    let href: String = reste[ferme + 2..fin].iter().collect();
    Some((libelle, href, fin + 1))
}

#[cfg(test)]
#[path = "analyse.test.rs"]
mod tests;
