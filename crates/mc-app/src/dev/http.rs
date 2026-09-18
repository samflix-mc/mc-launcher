//! Un serveur HTTP minuscule, pour le développement et rien d'autre.
//!
//! ## Pourquoi pas une bibliothèque
//!
//! Deux cents lignes de `tokio` contre une dépendance de plus dans un arbre
//! qu'on audite à chaque poussée. Ce serveur ne sert QUE le poste du
//! développeur : il écoute sur la boucle locale, il parle à un seul client, et
//! il n'a à résister à rien — ni à la charge, ni à un attaquant, puisqu'il
//! n'existe pas dans le binaire distribué.
//!
//! Ce qu'il sait faire est donc exactement ce qu'il faut : lire une requête,
//! rendre du JSON, et tenir un flux d'événements ouvert. Ce qu'il ne sait pas
//! faire — HTTPS, `keep-alive`, encodages de transfert — il ne le rencontrera
//! pas.
//!
//! ## Le découpage
//!
//! [`Requete`] est ce que le serveur a compris ; [`Reponse`] ce qu'il rend.
//! Les deux sont des structures ordinaires, ce qui rend le routage — qui, lui,
//! porte des décisions — éprouvable sans ouvrir de socket.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// Ce qu'une requête nous apprend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requete {
    pub methode: String,
    pub chemin: String,
    pub corps: String,
}

/// Ce qu'on rend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reponse {
    pub code: u16,
    pub type_mime: String,
    pub corps: String,
}

impl Reponse {
    pub fn json(corps: String) -> Reponse {
        Reponse {
            code: 200,
            type_mime: "application/json".to_string(),
            corps,
        }
    }

    /// Une erreur, dans la MÊME forme que celle du pont Tauri.
    ///
    /// `invoke` rejette avec la valeur d'erreur telle quelle — une chaîne — et
    /// le front la traite par `messageDErreur`. Rendre ici un objet d'une autre
    /// forme obligerait le front à distinguer deux transports, ce que tout ce
    /// module existe pour éviter.
    pub fn erreur(code: u16, message: &str) -> Reponse {
        Reponse {
            code,
            type_mime: "application/json".to_string(),
            corps: serde_json::to_string(message).unwrap_or_else(|_| "\"erreur\"".to_string()),
        }
    }

    pub fn vide() -> Reponse {
        Reponse::json("null".to_string())
    }
}

/// La ligne de requête et les en-têtes, analysés.
///
/// Rend `None` sur ce qu'on ne sait pas lire : une requête mal formée n'a pas
/// à faire tomber le serveur.
pub fn analyser(brut: &str) -> Option<(String, String, HashMap<String, String>)> {
    let (entete, _) = brut.split_once("\r\n\r\n").unwrap_or((brut, ""));
    let mut lignes = entete.lines();

    let premiere = lignes.next()?;
    let mut morceaux = premiere.split_whitespace();
    let methode = morceaux.next()?.to_string();
    let cible = morceaux.next()?.to_string();

    // La requête peut porter une chaîne de requête ; le routage n'en veut pas.
    let chemin = cible.split('?').next().unwrap_or(&cible).to_string();

    let mut entetes = HashMap::new();
    for ligne in lignes {
        if let Some((nom, valeur)) = ligne.split_once(':') {
            entetes.insert(nom.trim().to_ascii_lowercase(), valeur.trim().to_string());
        }
    }

    Some((methode, chemin, entetes))
}

/// Combien d'octets de corps la requête annonce.
///
/// Zéro quand l'en-tête manque ou ne se lit pas : un corps qu'on n'attend pas
/// vaut mieux qu'une attente qui ne se termine jamais.
pub fn longueur_annoncee(entetes: &HashMap<String, String>) -> usize {
    entetes
        .get("content-length")
        .and_then(|valeur| valeur.parse().ok())
        .unwrap_or(0)
}

/// Le texte d'une réponse, tel qu'il part sur la socket.
///
/// Les en-têtes d'origine croisée sont là parce que le front tourne sur le
/// serveur de développement d'Angular, donc sur un AUTRE port. Sans eux, le
/// navigateur refuse la réponse sans que rien n'apparaisse côté serveur.
pub fn rendre(reponse: &Reponse) -> String {
    format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        reponse.code,
        raison(reponse.code),
        reponse.type_mime,
        reponse.corps.len(),
        reponse.corps
    )
}

fn raison(code: u16) -> &'static str {
    match code {
        204 => "No Content",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

/// Ce que le serveur fait d'une requête.
pub type Routeur =
    Arc<dyn Fn(Requete) -> Pin<Box<dyn Future<Output = Reponse> + Send>> + Send + Sync>;

/// Écoute, et sert jusqu'à ce qu'on l'arrête.
///
/// `evenements` alimente le flux SSE : tout ce qui y est publié part vers les
/// clients abonnés à `/evenements`.
pub async fn servir(
    port: u16,
    routeur: Routeur,
    evenements: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let ecoute = TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(port, "serveur de développement à l'écoute");

    loop {
        let (flux, _) = ecoute.accept().await?;
        let routeur = Arc::clone(&routeur);
        let evenements = evenements.clone();
        tokio::spawn(async move {
            if let Err(erreur) = traiter(flux, routeur, evenements).await {
                tracing::debug!(erreur = %erreur, "connexion close");
            }
        });
    }
}

async fn traiter(
    mut flux: TcpStream,
    routeur: Routeur,
    evenements: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let mut tampon = Vec::new();
    let mut morceau = [0u8; 4096];

    // On lit jusqu'à la fin des en-têtes, puis exactement ce que
    // `Content-Length` annonce. Pas de `keep-alive` : un aller-retour par
    // connexion, ce qui suffit à un seul client et supprime tout un pan de
    // machine à états.
    let (methode, chemin, entetes) = loop {
        let lus = flux.read(&mut morceau).await?;
        if lus == 0 {
            return Ok(());
        }
        tampon.extend_from_slice(&morceau[..lus]);
        let texte = String::from_utf8_lossy(&tampon).into_owned();
        if texte.contains("\r\n\r\n")
            && let Some(analyse) = analyser(&texte)
        {
            break analyse;
        }
    };

    // Le préflight : le front tourne sur un autre port, et le navigateur
    // demande la permission avant d'envoyer du JSON.
    if methode == "OPTIONS" {
        let reponse = Reponse {
            code: 204,
            type_mime: "text/plain".to_string(),
            corps: String::new(),
        };
        flux.write_all(rendre(&reponse).as_bytes()).await?;
        return Ok(());
    }

    if chemin == "/evenements" {
        return diffuser(flux, evenements).await;
    }

    let attendu = longueur_annoncee(&entetes);
    let texte = String::from_utf8_lossy(&tampon).into_owned();
    let deja = texte.split_once("\r\n\r\n").map(|(_, c)| c).unwrap_or("");
    let mut corps = deja.as_bytes().to_vec();
    while corps.len() < attendu {
        let lus = flux.read(&mut morceau).await?;
        if lus == 0 {
            break;
        }
        corps.extend_from_slice(&morceau[..lus]);
    }

    let reponse = routeur(Requete {
        methode,
        chemin,
        corps: String::from_utf8_lossy(&corps).into_owned(),
    })
    .await;

    flux.write_all(rendre(&reponse).as_bytes()).await
}

/// Le flux d'événements, en SSE.
///
/// Remplace `listen()` de Tauri : le front s'y abonne par `EventSource` et
/// reçoit les mêmes charges utiles sous les mêmes noms.
async fn diffuser(
    mut flux: TcpStream,
    evenements: broadcast::Sender<String>,
) -> std::io::Result<()> {
    let entete = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: keep-alive\r\n\
         \r\n";
    flux.write_all(entete.as_bytes()).await?;

    let mut abonnement = evenements.subscribe();
    loop {
        match abonnement.recv().await {
            Ok(charge) => {
                flux.write_all(format!("data: {charge}\n\n").as_bytes())
                    .await?;
                flux.flush().await?;
            }
            // Un client lent a laissé passer des messages : on continue plutôt
            // que de le déconnecter. Un avancement perdu est sans conséquence
            // — le suivant porte l'état complet, pas un delta.
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "http.test.rs"]
mod tests;
