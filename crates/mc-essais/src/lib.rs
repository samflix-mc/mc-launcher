//! Un serveur HTTP local, pour éprouver ce qui parle au réseau.
//!
//! Tout ce que le launcher télécharge vient de cinq API — Mojang, Modrinth,
//! CurseForge, NeoForge, Adoptium — et la moitié du code non couvert du dépôt
//! est faite d'appels vers elles. Les vérifier demandait soit de sortir sur le
//! réseau à chaque `cargo test`, soit de rendre les réponses fabricables.
//!
//! Sortir sur le réseau était exclu : la CI tomberait sur une panne de
//! CurseForge, les quotas seraient consommés par les tests, et surtout rien ne
//! permettrait de provoquer les cas qui comptent — un 500 qui passe au second
//! essai, une clé refusée, un fichier tronqué, un JSON que personne ne publie
//! plus.
//!
//! D'où ce serveur : il écoute sur une adresse locale éphémère, rend ce qu'on
//! lui a dit de rendre, et note ce qu'on lui a demandé. Il parle juste assez
//! d'HTTP/1.1 pour que `reqwest` s'en satisfasse.
//!
//! ```no_run
//! # async fn exemple() {
//! let serveur = mc_essais::Serveur::neuf().await;
//! serveur.json("/v2/project/jei", r#"{"slug":"jei"}"#);
//! let url = serveur.url("/v2/project/jei");
//! # }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Ce que le serveur rendra pour un chemin.
#[derive(Clone)]
struct Reponse {
    code: u16,
    corps: Vec<u8>,
    type_contenu: String,
    /// Nombre de réponses en erreur à servir avant la vraie.
    ///
    /// C'est le seul moyen d'éprouver les réessais : une panne passagère de
    /// CDN ne se commande pas, et sans elle la boucle de réessai n'est jamais
    /// parcourue plus d'une fois.
    echecs_restants: u32,
}

/// Une requête reçue, telle qu'on veut pouvoir l'affirmer après coup.
#[derive(Clone, Debug)]
pub struct Requete {
    pub methode: String,
    pub chemin: String,
    pub entetes: HashMap<String, String>,
    pub corps: String,
}

impl Requete {
    /// Valeur d'un en-tête, casse ignorée — la clé de CurseForge passe par
    /// `x-api-key`, et rien ne garantit la casse au retour.
    pub fn entete(&self, nom: &str) -> Option<&str> {
        self.entetes
            .get(&nom.to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[derive(Default)]
struct Etat {
    routes: HashMap<String, Reponse>,
    recues: Vec<Requete>,
}

pub struct Serveur {
    adresse: SocketAddr,
    etat: Arc<Mutex<Etat>>,
}

impl Serveur {
    /// Ouvre un serveur sur un port libre de la boucle locale.
    pub async fn neuf() -> Serveur {
        let ecoute = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("aucun port libre sur la boucle locale");
        let adresse = ecoute.local_addr().unwrap();
        let etat = Arc::new(Mutex::new(Etat::default()));

        let partage = Arc::clone(&etat);
        tokio::spawn(async move {
            while let Ok((flux, _)) = ecoute.accept().await {
                let etat = Arc::clone(&partage);
                tokio::spawn(async move {
                    servir(flux, etat).await;
                });
            }
        });

        Serveur { adresse, etat }
    }

    /// Racine du serveur, à substituer à l'URL de l'API visée.
    pub fn base(&self) -> String {
        format!("http://{}", self.adresse)
    }

    pub fn url(&self, chemin: &str) -> String {
        format!("{}{chemin}", self.base())
    }

    /// Répond du JSON sur ce chemin.
    pub fn json(&self, chemin: &str, corps: &str) -> &Self {
        self.pose(chemin, 200, corps.as_bytes(), "application/json")
    }

    /// Répond des octets bruts — un jar, un fichier d'assets.
    pub fn octets(&self, chemin: &str, corps: &[u8]) -> &Self {
        self.pose(chemin, 200, corps, "application/octet-stream")
    }

    /// Répond un code d'erreur, sans corps utile.
    pub fn code(&self, chemin: &str, code: u16) -> &Self {
        self.pose(chemin, code, b"erreur", "text/plain")
    }

    /// Répond un code d'erreur avec un corps choisi — certaines API disent
    /// dans le corps ce que leur code ne dit pas.
    pub fn code_avec(&self, chemin: &str, code: u16, corps: &str) -> &Self {
        self.pose(chemin, code, corps.as_bytes(), "application/json")
    }

    /// Échoue `combien` fois, puis répond normalement.
    pub fn echoue_puis(&self, chemin: &str, combien: u32, corps: &str) -> &Self {
        self.pose(chemin, 200, corps.as_bytes(), "application/json");
        if let Some(reponse) = self.etat.lock().unwrap().routes.get_mut(chemin) {
            reponse.echecs_restants = combien;
        }
        self
    }

    fn pose(&self, chemin: &str, code: u16, corps: &[u8], type_contenu: &str) -> &Self {
        self.etat.lock().unwrap().routes.insert(
            chemin.to_string(),
            Reponse {
                code,
                corps: corps.to_vec(),
                type_contenu: type_contenu.to_string(),
                echecs_restants: 0,
            },
        );
        self
    }

    /// Tout ce qui a été demandé, dans l'ordre.
    pub fn recues(&self) -> Vec<Requete> {
        self.etat.lock().unwrap().recues.clone()
    }

    /// Nombre de requêtes reçues sur un chemin, préfixe de requête compris.
    pub fn appels(&self, chemin: &str) -> usize {
        self.recues()
            .iter()
            .filter(|r| r.chemin == chemin || r.chemin.starts_with(&format!("{chemin}?")))
            .count()
    }
}

async fn servir(mut flux: tokio::net::TcpStream, etat: Arc<Mutex<Etat>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut brut = Vec::new();
    let mut tampon = [0u8; 4096];
    // L'en-tête se termine par une ligne vide ; le corps, s'il y en a un, suit
    // sur la longueur annoncée.
    let separation = loop {
        let lu = match flux.read(&mut tampon).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        brut.extend_from_slice(&tampon[..lu]);
        if let Some(position) = trouver(&brut, b"\r\n\r\n") {
            break position;
        }
    };

    let entete = String::from_utf8_lossy(&brut[..separation]).to_string();
    let mut lignes = entete.lines();
    let premiere = lignes.next().unwrap_or_default();
    let mut morceaux = premiere.split_whitespace();
    let methode = morceaux.next().unwrap_or("GET").to_string();
    let chemin = morceaux.next().unwrap_or("/").to_string();

    let mut entetes = HashMap::new();
    for ligne in lignes {
        if let Some((nom, valeur)) = ligne.split_once(':') {
            entetes.insert(nom.trim().to_ascii_lowercase(), valeur.trim().to_string());
        }
    }

    let attendu: usize = entetes
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut corps = brut[separation + 4..].to_vec();
    while corps.len() < attendu {
        match flux.read(&mut tampon).await {
            Ok(0) | Err(_) => break,
            Ok(n) => corps.extend_from_slice(&tampon[..n]),
        }
    }

    // Le chemin sert de clé sans sa requête : les appelants y mettent des
    // paramètres qu'on ne veut pas avoir à recopier à l'identique.
    let cle = chemin.split('?').next().unwrap_or(&chemin).to_string();
    let (code, charge, type_contenu) = {
        let mut etat = etat.lock().unwrap();
        etat.recues.push(Requete {
            methode,
            chemin: chemin.clone(),
            entetes,
            corps: String::from_utf8_lossy(&corps).to_string(),
        });
        match etat.routes.get_mut(&cle) {
            None => (404, b"rien ici".to_vec(), "text/plain".to_string()),
            Some(reponse) if reponse.echecs_restants > 0 => {
                reponse.echecs_restants -= 1;
                (500, b"panne passagere".to_vec(), "text/plain".to_string())
            }
            Some(reponse) => (
                reponse.code,
                reponse.corps.clone(),
                reponse.type_contenu.clone(),
            ),
        }
    };

    let tete = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {type_contenu}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        raison(code),
        charge.len()
    );
    let _ = flux.write_all(tete.as_bytes()).await;
    let _ = flux.write_all(&charge).await;
    let _ = flux.flush().await;
}

fn trouver(foin: &[u8], aiguille: &[u8]) -> Option<usize> {
    foin.windows(aiguille.len()).position(|f| f == aiguille)
}

fn raison(code: u16) -> &'static str {
    match code {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Status",
    }
}
