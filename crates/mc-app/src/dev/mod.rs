//! Le launcher sans sa fenêtre, pour travailler l'interface.
//!
//! ## Le problème que ce module résout
//!
//! Le front ne peut être regardé que dans la fenêtre Tauri, parce que c'est là
//! que `invoke` répond. Or dans cette fenêtre il n'y a ni rechargement à
//! chaud, ni inspecteur en production, ni moyen de mettre l'écran dans un état
//! choisi. Chaque essai coûte un build complet, et les états rares — trois
//! mods introuvables, une installation à mi-parcours — ne se provoquent pas.
//!
//! Ce module sert les MÊMES commandes sur HTTP. Le front tourne alors dans un
//! navigateur ordinaire, avec son rechargement à chaud, ses outils de
//! développement, et un état qu'on choisit en une requête.
//!
//! ## Ce qui garantit qu'il ne part pas en production
//!
//! Il est derrière la feature `dev-serveur`, qui n'est pas activée par défaut,
//! et il n'est compilé que dans un binaire séparé — `mc-dev-serveur`, déclaré
//! avec `required-features`. `cargo tauri build` ne l'active pas : le code
//! n'est donc pas « désactivé » dans le binaire du launcher, il n'y est pas.
//!
//! ## Ce qui est vrai et ce qui est simulé
//!
//! Vrai : `marque`, `chemin`, `reglages`, `enregistrer_reglages`. Elles ne
//! touchent ni au réseau ni au disque du jeu, et les voir mentir n'apprendrait
//! rien — `enregistrer_reglages` écrit donc réellement, ce qui est exactement
//! ce qu'on veut éprouver, puisque c'est là que les bornes s'appliquent.
//!
//! Simulé : tout ce qui authentifie, installe ou lance. Voir [`scenario`].

pub mod http;
pub mod scenario;

use std::sync::{Arc, Mutex};

use serde_json::json;
use tokio::sync::broadcast;

use http::{Reponse, Requete};
use scenario::Etat;

/// Le port par défaut.
///
/// 1421 parce que le serveur d'Angular prend 1420 : les deux tournent
/// ensemble, et voir les deux nombres se suivre évite d'avoir à se rappeler
/// lequel est lequel.
pub const PORT: u16 = 1421;

/// Ce que le serveur garde entre deux requêtes.
pub struct Contexte {
    pub etat: Mutex<Etat>,
    pub evenements: broadcast::Sender<String>,
}

impl Contexte {
    pub fn neuf(depart: Etat) -> Arc<Contexte> {
        let (evenements, _) = broadcast::channel(64);
        Arc::new(Contexte {
            etat: Mutex::new(depart),
            evenements,
        })
    }

    fn lire(&self) -> Etat {
        *self.etat.lock().expect("état non empoisonné")
    }

    /// Publie un événement, dans la forme que le front attend.
    ///
    /// Le nom accompagne la charge utile : SSE n'a qu'un seul flux, là où
    /// `listen()` en a un par nom. Le front démultiplexe.
    fn emettre(&self, nom: &str, charge: serde_json::Value) {
        let message = json!({ "evenement": nom, "charge": charge }).to_string();
        // Une erreur ici veut seulement dire que personne n'écoute.
        let _ = self.evenements.send(message);
    }
}

/// Le routage, séparé du transport.
///
/// Une fonction ordinaire qui prend une requête et rend une réponse : c'est ce
/// qui la rend éprouvable sans ouvrir de socket, et c'est là que vivent les
/// seules décisions du module.
pub async fn router(contexte: Arc<Contexte>, requete: Requete) -> Reponse {
    let etat = contexte.lire();

    // Le scénario se change en une requête — c'est tout l'intérêt.
    if let Some(nom) = requete.chemin.strip_prefix("/scenario/") {
        return match Etat::depuis_nom(nom) {
            Some(nouveau) => {
                *contexte.etat.lock().expect("état non empoisonné") = nouveau;
                contexte.emettre(
                    crate::cinematique::EVENEMENT_AVANCEMENT,
                    serde_json::to_value(nouveau.avancement()).unwrap_or(serde_json::Value::Null),
                );
                Reponse::json(json!({ "scenario": nom }).to_string())
            }
            None => Reponse::erreur(404, &format!("scénario inconnu : {nom}")),
        };
    }

    let Some(nom) = requete.chemin.strip_prefix("/commande/") else {
        return accueil(etat);
    };

    match nom {
        // --- Ce qui appelle la vraie implémentation --------------------------
        "marque" => valeur(&crate::commandes::marque()),
        "chemin" => valeur(&scenario::chemin()),
        "reglages" => valeur(&crate::commandes::reglages::reglages()),
        "enregistrer_reglages" => match argument(&requete.corps, "reglages") {
            Some(recu) => match serde_json::from_value(recu) {
                Ok(reglages) => {
                    resultat(crate::commandes::reglages::enregistrer_reglages(reglages))
                }
                Err(erreur) => Reponse::erreur(500, &format!("réglages illisibles : {erreur}")),
            },
            None => Reponse::erreur(500, "il manque l'argument « reglages »"),
        },

        // --- Ce que le scénario décide ---------------------------------------
        "statut" => valeur(&etat.compte()),
        "etat_du_pack" => valeur(&etat.pack()),
        "nouvelles" => valeur(&etat.fil()),
        "verifier_les_fichiers" => valeur(&Vec::<String>::new()),

        "connexion" => connexion(&contexte).await,

        "deconnexion" => {
            *contexte.etat.lock().expect("état non empoisonné") = Etat::Deconnecte;
            Reponse::vide()
        }

        "jouer" => jouer(&contexte, etat, true).await,
        "installer" => jouer(&contexte, etat, false).await,

        // --- Ce qui n'a pas de sens hors de la fenêtre ------------------------
        //
        // `ecran` rend `null`, ce que la page Configuration sait traiter : elle
        // retombe sur sa liste fixe de résolutions. `ouvrir_dossier` et
        // `front_pret` ne font rien — il n'y a ni explorateur de fichiers à
        // ouvrir, ni écran de démarrage à refermer.
        "ecran" => Reponse::json("null".to_string()),
        "ouvrir_dossier" | "front_pret" => Reponse::vide(),

        // Les deux temps de la fenêtre de connexion. Dans un navigateur il n'y
        // a qu'un onglet : il n'y a rien à ouvrir ni à refermer, et c'est le
        // routeur du front qui emmène d'une page à l'autre. Elles répondent
        // quand même — une commande inconnue rendrait un 404 que le front
        // ouvrirait en incident, sur un geste qui n'a simplement pas lieu ici.
        "ouvrir_connexion" | "principale_prete" => Reponse::vide(),

        // Le front journalise ici aussi, sous l'étiquette « navigateur » : il
        // n'y a pas de fenêtre à interroger, et la séquence reste lisible dans
        // le même flux que le reste.
        "journal" => {
            let niveau = texte(&requete.corps, "niveau").unwrap_or_else(|| "info".to_string());
            let message = texte(&requete.corps, "message").unwrap_or_default();
            tracing::info!(target: "front", fenetre = "navigateur", niveau = %niveau, "{message}");
            Reponse::vide()
        }

        // Le plancher de l'écran « connecté », tenu ici aussi.
        //
        // Dans la fenêtre, c'est `fenetres::connexion_reussie` qui le tient —
        // deux secondes au moins, le temps qu'on lise son pseudo et qu'on
        // comprenne que ça a marché. Sans le reproduire, cet écran passerait en
        // une image dans un navigateur, et l'on ne pourrait pas le travailler :
        // c'est exactement ce que ce serveur existe pour rendre observable.
        "connexion_reussie" => {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            Reponse::vide()
        }

        _ => Reponse::erreur(404, &format!("commande inconnue : {nom}")),
    }
}

/// La connexion, jouée comme la vraie : le code d'abord, le compte après.
///
/// Le délai n'est pas une coquetterie. Chez Microsoft, entre le moment où le
/// joueur autorise et celui où le launcher s'en aperçoit, il s'écoule le temps
/// d'un intervalle de sondage — mesuré à environ quatre secondes. C'est
/// précisément l'attente que l'interface doit savoir habiter, donc celle qu'il
/// faut pouvoir regarder.
async fn connexion(contexte: &Arc<Contexte>) -> Reponse {
    contexte.emettre(
        crate::commandes::EVENEMENT_CODE,
        json!({
            "code": "FKRD-QXZB",
            "url": "https://www.microsoft.com/link",
            "urlDirecte": "https://www.microsoft.com/link?otc=FKRD-QXZB",
        }),
    );

    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    *contexte.etat.lock().expect("état non empoisonné") = Etat::RienInstalle;
    valeur(&Etat::RienInstalle.compte())
}

/// Une partie, précédée de son installation quand il y a lieu.
///
/// Les étapes défilent pour de bon, avec des octets qui montent : c'est le
/// seul moyen de voir si la barre, les libellés et le débit se tiennent
/// pendant plusieurs secondes, et non sur une capture figée.
async fn jouer(contexte: &Arc<Contexte>, etat: Etat, avec_partie: bool) -> Reponse {
    use crate::phase::Phase;

    let etapes = [
        (Phase::Pack, "Lecture du manifeste"),
        (Phase::Chargeur, "NeoForge 21.1.250"),
        (Phase::Minecraft, "Client, bibliothèques et assets"),
        (Phase::Java, "Temurin 21"),
        (Phase::NeoForge, "Installateur officiel"),
        (Phase::Mods, "128 mods"),
        (Phase::Verrou, "Verrou écrit"),
    ];

    for (rang, (phase, note)) in etapes.iter().enumerate() {
        let total = 840_000_000u64;
        let octets = total * (rang as u64 + 1) / etapes.len() as u64;
        contexte.emettre(
            crate::cinematique::EVENEMENT_AVANCEMENT,
            json!({
                "phase": phase,
                "achevee": false,
                "note": note,
                "fichier": "sodium-neoforge-0.6.13.jar",
                "octets": octets,
                "total": total,
                "fichiers": (rang + 1) * 18,
                "fichiersTotal": 128,
                "actif": true,
                "debit": 8_400_000,
                "restant": (etapes.len() - rang - 1) as u64 * 9,
            }),
        );
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    }

    // La partie elle-même — SEULEMENT si le geste est « jouer ». Le bouton
    // d'installation s'arrête ici : c'est tout l'objet de la séparation.
    if avec_partie {
        contexte.emettre(
            crate::cinematique::EVENEMENT_AVANCEMENT,
            json!({ "phase": Phase::Lancement, "achevee": false, "actif": false,
                    "octets": 0, "total": 0, "fichiers": 0, "fichiersTotal": 0, "debit": 0 }),
        );
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    *contexte.etat.lock().expect("état non empoisonné") = Etat::PretAJouer;
    contexte.emettre(
        crate::cinematique::EVENEMENT_AVANCEMENT,
        serde_json::to_value(Etat::PretAJouer.avancement()).unwrap_or(serde_json::Value::Null),
    );

    valeur(&etat.partie(avec_partie))
}

/// Ce que le serveur dit de lui-même, pour qui ouvre l'adresse à la main.
fn accueil(etat: Etat) -> Reponse {
    let courant = Etat::TOUS
        .iter()
        .find(|(_, connu)| *connu == etat)
        .map(|(nom, _)| *nom)
        .unwrap_or("inconnu");

    valeur(&scenario::Accueil {
        scenario: courant.to_string(),
        scenarios: Etat::TOUS.iter().map(|(nom, _)| nom.to_string()).collect(),
        commandes: vec![
            "marque",
            "chemin",
            "statut",
            "connexion",
            "deconnexion",
            "etat_du_pack",
            "jouer",
            "installer",
            "verifier_les_fichiers",
            "nouvelles",
            "reglages",
            "enregistrer_reglages",
            "ecran",
            "ouvrir_dossier",
            "front_pret",
            "ouvrir_connexion",
            "connexion_reussie",
            "principale_prete",
            "journal",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
    })
}

/// Déballe un `Result`, EXACTEMENT comme le pont Tauri le fait.
///
/// **C'est le seul endroit où ce serveur pouvait mentir sur la forme de ce
/// qu'il rend, et il a menti.** `#[tauri::command]` enveloppe une commande qui
/// rend un `Result` : le succès part comme la valeur nue, l'erreur rejette la
/// promesse. Sérialiser le `Result` tel quel donne `{"Ok": {…}}` — une réponse
/// qui a l'air d'une réussite, qui porte un code 200, et dont le front lit un
/// champ qui n'existe pas.
///
/// Le symptôme est resté longtemps illisible : la page de configuration
/// enregistrait, recevait un objet d'une forme qu'elle ne connaissait pas, et
/// ouvrait un incident par frappe de curseur — trois cent vingt-six en une
/// session. Rien dans le serveur ne le disait, puisque de son point de vue tout
/// s'était bien passé.
pub fn resultat<T, E>(resultat: Result<T, E>) -> Reponse
where
    T: serde::Serialize,
    E: serde::Serialize,
{
    match resultat {
        Ok(valeur_rendue) => valeur(&valeur_rendue),
        // L'erreur est SÉRIALISÉE, pas formatée : c'est ce que fait le pont,
        // qui rejette avec la valeur d'erreur telle quelle. `Erreur` est un
        // newtype sur une chaîne, donc cela rend une chaîne JSON — la forme
        // exacte que `messageDErreur` sait lire des deux côtés.
        Err(erreur) => match serde_json::to_string(&erreur) {
            Ok(corps) => Reponse {
                code: 500,
                type_mime: "application/json".to_string(),
                corps,
            },
            Err(cause) => Reponse::erreur(500, &format!("erreur non sérialisable : {cause}")),
        },
    }
}

/// Sérialise, ou rend l'échec plutôt que de le taire.
fn valeur<T: serde::Serialize>(valeur: &T) -> Reponse {
    match serde_json::to_string(valeur) {
        Ok(corps) => Reponse::json(corps),
        Err(erreur) => Reponse::erreur(500, &format!("réponse non sérialisable : {erreur}")),
    }
}

/// Un argument nommé, dans le corps JSON.
///
/// Le front envoie ce que `invoke` enverrait : un objet dont les clés sont les
/// noms des paramètres. On les lit de la même façon.
pub fn argument(corps: &str, nom: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(corps)
        .ok()?
        .get(nom)
        .cloned()
}

/// Le même, quand l'argument attendu est une chaîne.
///
/// Un argument absent ou d'un autre type rend `None` plutôt que d'échouer : ce
/// serveur sert à travailler l'interface, et une ligne de journal mal formée ne
/// doit pas interrompre le geste qu'on était en train d'observer.
pub fn texte(corps: &str, nom: &str) -> Option<String> {
    argument(corps, nom)?.as_str().map(str::to_owned)
}

#[cfg(test)]
#[path = "dev.test.rs"]
mod tests;
