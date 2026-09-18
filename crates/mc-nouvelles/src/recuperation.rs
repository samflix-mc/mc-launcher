//! Aller chercher le fil, garder une copie, et rapatrier les images.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::contrat::{Billet, Fil, FilBrut, SCHEMA, date_valide, ordonner};

/// Taille maximale d'une image de billet.
///
/// **Une borne, et il en fallait une.** `Downloader::bytes` n'a AUCUNE limite
/// et accumule le corps entier en mémoire, sous un délai de trois cents
/// secondes. Un hôte fautif — ou compromis — servirait un fichier de plusieurs
/// gigaoctets, et le launcher grossirait jusqu'à se faire tuer par le système,
/// sans un message.
///
/// **La borne protège la mémoire, pas l'affichage.** Mesuré le 18 septembre
/// 2026 dans la fenêtre, sous WebKitGTK et DMA-BUF désactivé : un `data:` de
/// **567 kio se décode et s'affiche en 5 ms**. Le transport n'est donc pas ce
/// qui justifie ce plafond, et le relever ne rendrait pas la page lente — cela
/// rendrait seulement le launcher vulnérable à un hôte qui servirait un
/// fichier sans fin.
pub const IMAGE_MAX: usize = 512 * 1024;

/// Combien d'images on rapatrie pour un fil.
///
/// Huit billets illustrés font déjà quatre mégaoctets ; au-delà, l'ouverture
/// de la page coûterait plus que ce qu'elle montre.
pub const IMAGES_MAX: usize = 8;

/// L'adresse du fil, DÉRIVÉE de celle du pack.
///
/// Le même motif que `lock_url_for` : recopier les trois adresses publiées
/// referait exactement le défaut que leur commentaire raconte — une
/// préproduction qui télécharge le contenu de la production, et n'éprouve donc
/// rien de ce qu'elle est censée éprouver.
pub fn url_du_fil(url_du_pack: &str) -> String {
    match url_du_pack.rsplit_once('/') {
        Some((base, _)) => format!("{base}/nouvelles.json"),
        None => "nouvelles.json".to_string(),
    }
}

/// Le fil, du réseau ou du cache.
///
/// Ne rend une erreur que si les deux échouent : une page de news vide vaut
/// mieux qu'une fenêtre qui refuse de s'ouvrir, mais une page qui ment sur sa
/// fraîcheur serait pire que les deux.
pub async fn charger(url_du_pack: &str, cache: &Path, dl: &mc_dl::Downloader) -> Result<Fil> {
    let url = url_du_fil(url_du_pack);
    let copie = cache.join("nouvelles.json");

    let octets = match dl.bytes(&url).await {
        Ok(octets) => {
            // Mis en cache seulement après avoir été RELU : une réponse
            // tronquée ou une page d'erreur HTML remplaceraient sinon un fil
            // valide par du vide.
            if lire(&octets, &url).is_ok()
                && let Err(erreur) = ecrire_le_cache(&copie, &octets)
            {
                tracing::warn!(erreur = %erreur, "copie du fil non écrite");
            }
            octets
        }
        // Un hôte qui RÉPOND « ce n'est pas là » ne publie pas de fil : la
        // page s'ouvre vide, et c'est la bonne réponse. Se rabattre ici sur la
        // copie afficherait des billets que quelqu'un a retirés exprès.
        Err(erreur) if erreur.downcast_ref::<mc_dl::Absent>().is_some() => {
            tracing::info!(url, "aucun fil publié sur cet hôte");
            return Ok(Fil {
                billets: Vec::new(),
                hors_ligne: false,
                ecartes: Vec::new(),
            });
        }
        Err(erreur) => {
            tracing::warn!(url, erreur = %erreur, "fil injoignable, repli sur la copie");
            let copie_lue = std::fs::read(&copie)
                .with_context(|| format!("fil {url} injoignable, et aucune copie"))?;
            let mut fil = lire(&copie_lue, &url)?;
            fil.hors_ligne = true;
            return Ok(fil);
        }
    };

    lire(&octets, &url)
}

fn ecrire_le_cache(copie: &Path, octets: &[u8]) -> Result<()> {
    if let Some(parent) = copie.parent() {
        std::fs::create_dir_all(parent)?;
    }
    mc_dl::write_atomic(copie, octets)
}

/// Analyse un fil, billet par billet.
///
/// Un billet fautif est écarté SEUL. C'est la différence entre « la page des
/// news a un trou » et « la page des news est vide », et la seconde se lit
/// comme une panne du launcher — alors que la cause est une virgule dans un
/// fichier qu'on ne contrôle pas au même rythme que le code.
pub fn lire(octets: &[u8], url_du_fil: &str) -> Result<Fil> {
    let brut: FilBrut = serde_json::from_slice(octets).context("fil de nouvelles illisible")?;

    let mut retenus = Vec::new();
    let mut ecartes = Vec::new();

    if brut.schema != SCHEMA {
        // Dans le COMPTE RENDU, et pas seulement dans le journal.
        //
        // Un fil servi sous un autre schéma se lit quand même — les champs
        // qu'on connaît se relisent — mais il faut que quelqu'un l'apprenne.
        // Le journal seul ne suffit pas : personne ne l'ouvre tant que rien
        // n'a l'air cassé, et c'est précisément le cas ici.
        //
        // La page des nouvelles affiche déjà `ecartes` dans un repli discret :
        // c'est exactement le bon endroit, et cela ne coûte rien à personne.
        ecartes.push(format!(
            "le fil annonce le schéma {} au lieu de {SCHEMA} : ce qui se lit est gardé",
            brut.schema
        ));
    }

    for billet in brut.billets {
        if !date_valide(&billet.date) {
            ecartes.push(format!(
                "{} : date « {} » hors du format attendu (RFC 3339 en UTC)",
                billet.id, billet.date
            ));
            continue;
        }
        if billet.titre.trim().is_empty() {
            ecartes.push(format!("{} : titre vide", billet.id));
            continue;
        }

        let image = billet.image.as_deref().and_then(|image| {
            match crate::liens::image_absolue(image, url_du_fil) {
                Some(absolue) => Some(absolue),
                None => {
                    ecartes.push(format!(
                        "{} : image « {image} » hors de l'hôte du fil, ignorée",
                        billet.id
                    ));
                    None
                }
            }
        });

        retenus.push(Billet {
            corps: crate::analyse::analyser(&billet.corps, url_du_fil),
            id: billet.id,
            titre: billet.titre,
            date: billet.date,
            epinglee: billet.epinglee,
            image,
        });
    }

    for ecart in &ecartes {
        tracing::warn!(ecart, "billet écarté du fil");
    }

    Ok(Fil {
        billets: ordonner(retenus),
        hors_ligne: false,
        ecartes,
    })
}

/// Rapatrie une image dans le cache, et rend son chemin local.
///
/// ## Stockage et transport séparés
///
/// L'image est rangée en FICHIER, nommée par l'empreinte de son URL. Le
/// `data:` que la fenêtre consomme n'est fabriqué qu'à la demande, par
/// [`en_data_url`].
///
/// Garder les images en base64 dans le cache JSON ferait qu'un fil de huit
/// billets illustrés pèserait trois mégaoctets et demi, relus et resérialisés
/// à chaque ouverture de la page — pour un rendu qui ne montre que la première
/// avant qu'on ne fasse défiler.
pub async fn rapatrier(url: &str, cache: &Path, dl: &mc_dl::Downloader) -> Result<PathBuf> {
    let destination = cache.join("images").join(nom_de_fichier(url));
    if destination.is_file() {
        return Ok(destination);
    }

    let octets = dl.bytes(url).await?;
    if octets.len() > IMAGE_MAX {
        // En kio et non en octets : le message sera lu dans un journal par
        // quelqu'un qui cherche pourquoi une illustration manque, et « 524289
        // au-delà de 524288 » se compare moins vite que « 513 au-delà de 512 ».
        anyhow::bail!(
            "image {url} : {} kio, au-delà des {} kio autorisés",
            octets.len().div_ceil(1024),
            IMAGE_MAX / 1024
        );
    }
    if type_mime(&octets).is_none() {
        anyhow::bail!("image {url} : ces octets ne sont pas une image reconnue");
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    mc_dl::write_atomic(&destination, &octets)?;
    Ok(destination)
}

/// Le nom de fichier d'une image mise en cache.
///
/// L'empreinte de l'URL, et non son dernier segment : deux billets peuvent
/// porter une `image.webp` chacun, et le second écraserait le premier.
fn nom_de_fichier(url: &str) -> String {
    mc_dl::sha512_of_bytes(url.as_bytes())[..32].to_string()
}

/// Le type MIME, DÉDUIT DES OCTETS et non de l'extension.
///
/// Une extension vient de l'URL, donc de l'hôte, donc de quelque chose qu'on
/// ne contrôle pas au même rythme que le code. Les nombres magiques, eux, sont
/// dans le fichier.
pub fn type_mime(octets: &[u8]) -> Option<&'static str> {
    if octets.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if octets.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if octets.starts_with(b"GIF87a") || octets.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    // WebP : « RIFF » puis quatre octets de taille puis « WEBP ».
    //
    // `>= 12` et non `> 12` : douze octets SONT exactement la signature, et
    // c'est tout ce qu'il faut pour l'identifier. Exiger un treizième
    // refuserait un en-tête complet parce qu'il lui manque un octet de
    // contenu — ce qui n'est pas la question posée à cette fonction.
    if octets.len() >= 12 && octets.starts_with(b"RIFF") && &octets[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

/// Fabrique le `data:` que la fenêtre consomme, à la demande.
pub fn en_data_url(chemin: &Path) -> Result<String> {
    let octets = std::fs::read(chemin)?;
    let mime = type_mime(&octets)
        .with_context(|| format!("{} n'est plus une image reconnue", chemin.display()))?;
    Ok(format!("data:{mime};base64,{}", base64(&octets)))
}

/// Base64 standard, sans dépendance.
///
/// Trente lignes valent mieux qu'une crate de plus dans un binaire qu'on
/// distribue : c'est un encodage figé depuis 1987, et celui-ci ne sert qu'à
/// une chose.
fn base64(octets: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut sortie = String::with_capacity(octets.len().div_ceil(3) * 4);

    for morceau in octets.chunks(3) {
        let b = [
            morceau[0],
            *morceau.get(1).unwrap_or(&0),
            *morceau.get(2).unwrap_or(&0),
        ];
        // `from_be_bytes` et non trois décalages assemblés par `|`.
        //
        // Le calcul est le même, la lecture est meilleure, et il se trouve
        // que c'est aussi la seule forme qui ne laisse rien à muter : sur des
        // octets rangés dans des champs de bits disjoints, `|` et `^` rendent
        // exactement le même nombre. Un mutant qui remplace l'un par l'autre
        // est donc ÉQUIVALENT — aucun test ne peut le tuer, et il resterait
        // au compte des survivants pour toujours.
        //
        // Le réflexe serait de l'exclure par un attribut. Mais un mutant
        // équivalent dit souvent qu'une expression fait un détour : ici, le
        // détour était d'assembler à la main ce que la bibliothèque standard
        // assemble mieux.
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        sortie.push(ALPHABET[(n >> 18) as usize & 63] as char);
        sortie.push(ALPHABET[(n >> 12) as usize & 63] as char);
        sortie.push(if morceau.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        sortie.push(if morceau.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    sortie
}

#[cfg(test)]
#[path = "recuperation.test.rs"]
mod tests;
