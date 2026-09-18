use super::{IMAGE_MAX, base64, charger, en_data_url, lire, rapatrier, type_mime, url_du_fil};

struct Atelier {
    racine: std::path::PathBuf,
}

impl Atelier {
    fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-nouvelles-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }
}

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

const FIL: &str = "https://mc-launcher.ggy.info/pack/nouvelles.json";

fn corps(billets: &str) -> String {
    format!(r#"{{"schema":1,"billets":[{billets}]}}"#)
}

fn un_billet(id: &str, date: &str) -> String {
    format!(r#"{{"id":"{id}","titre":"Titre","date":"{date}","corps":"Bonjour."}}"#)
}

/// L'adresse du fil est DÉRIVÉE de celle du pack, jamais recopiée.
///
/// Le même motif que `lock_url_for` : recopier les trois adresses publiées
/// referait le défaut que leur commentaire raconte — une préproduction qui
/// télécharge le contenu de la production, et n'éprouve donc rien de ce
/// qu'elle est censée éprouver.
#[test]
fn l_adresse_du_fil_se_derive_de_celle_du_pack() {
    assert_eq!(
        url_du_fil("https://mc-launcher.ggy.info/pack/samflix.json"),
        "https://mc-launcher.ggy.info/pack/nouvelles.json"
    );
    assert_eq!(
        url_du_fil("https://mc-launcher-dev.ggy.info/pack/samflix.json"),
        "https://mc-launcher-dev.ggy.info/pack/nouvelles.json"
    );
}

/// Un billet fautif est écarté SEUL.
///
/// C'est la différence entre « la page des news a un trou » et « la page des
/// news est vide », et la seconde se lit comme une panne du launcher — alors
/// que la cause est une virgule dans un fichier qu'on ne contrôle pas au même
/// rythme que le code.
#[test]
fn un_billet_fautif_n_emporte_pas_les_autres() {
    let brut = corps(&format!(
        "{},{},{}",
        un_billet("bon-1", "2026-09-01T00:00:00Z"),
        un_billet("date-cassee", "hier"),
        un_billet("bon-2", "2026-09-02T00:00:00Z"),
    ));

    let fil = lire(brut.as_bytes(), FIL).expect("le fil se lit");

    assert_eq!(fil.billets.len(), 2, "{:?}", fil.ecartes);
    assert_eq!(fil.ecartes.len(), 1);
    assert!(fil.ecartes[0].contains("date-cassee"), "{:?}", fil.ecartes);
}

#[test]
fn un_titre_vide_ecarte_le_billet() {
    let brut = corps(r#"{"id":"vide","titre":"  ","date":"2026-09-01T00:00:00Z","corps":"x"}"#);
    let fil = lire(brut.as_bytes(), FIL).unwrap();
    assert!(fil.billets.is_empty());
    assert_eq!(fil.ecartes.len(), 1);
}

/// Une image hors de l'hôte n'écarte pas le billet : elle est simplement
/// ignorée. Perdre tout un billet parce que son illustration est mal rangée
/// serait disproportionné.
#[test]
fn une_image_d_ailleurs_ne_fait_pas_perdre_le_billet() {
    let brut = corps(
        r#"{"id":"x","titre":"T","date":"2026-09-01T00:00:00Z","image":"https://evil.example/i.webp","corps":"x"}"#,
    );
    let fil = lire(brut.as_bytes(), FIL).unwrap();

    assert_eq!(fil.billets.len(), 1);
    assert_eq!(fil.billets[0].image, None);
    assert_eq!(fil.ecartes.len(), 1);
}

#[test]
fn une_image_relative_devient_absolue() {
    let brut = corps(
        r#"{"id":"x","titre":"T","date":"2026-09-01T00:00:00Z","image":"s3.webp","corps":"x"}"#,
    );
    let fil = lire(brut.as_bytes(), FIL).unwrap();
    assert_eq!(
        fil.billets[0].image.as_deref(),
        Some("https://mc-launcher.ggy.info/pack/s3.webp")
    );
}

#[test]
fn un_fil_illisible_rend_une_erreur() {
    assert!(lire(b"{pas du JSON", FIL).is_err());
}

// --- Réseau et repli -------------------------------------------------------

#[tokio::test]
async fn le_fil_se_recupere_et_se_met_en_cache() {
    let atelier = Atelier::neuf("fil-reseau");
    let serveur = mc_essais::Serveur::neuf().await;
    let brut = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    serveur.json("/pack/nouvelles.json", &brut);
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let fil = charger(
        &format!("{}/pack/samflix.json", serveur.base()),
        &atelier.racine,
        &dl,
    )
    .await
    .expect("le fil se charge");

    assert_eq!(fil.billets.len(), 1);
    assert!(!fil.hors_ligne);
    assert!(
        atelier.racine.join("nouvelles.json").is_file(),
        "pas de copie"
    );
}

/// Sans réseau, la copie prend le relais et le dit. Une page de news d'hier
/// vaut mieux qu'une page vide ; une page qui mentirait sur sa fraîcheur
/// serait pire que les deux.
#[tokio::test]
async fn sans_reseau_la_copie_prend_le_relais_et_le_dit() {
    let atelier = Atelier::neuf("fil-hors-ligne");
    let brut = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    std::fs::write(atelier.racine.join("nouvelles.json"), &brut).unwrap();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let fil = charger("http://127.0.0.1:1/pack/samflix.json", &atelier.racine, &dl)
        .await
        .expect("la copie sert");

    assert_eq!(fil.billets.len(), 1);
    assert!(fil.hors_ligne, "la fenêtre croirait le fil à jour");
}

/// Sans réseau ET sans copie, une erreur franche : c'est la seule situation où
/// l'on n'a rien à montrer, et la page doit le dire plutôt que d'afficher un
/// vide qu'on prendrait pour « aucune nouvelle ».
#[tokio::test]
async fn sans_reseau_ni_copie_l_erreur_est_franche() {
    let atelier = Atelier::neuf("fil-rien");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    assert!(
        charger("http://127.0.0.1:1/pack/samflix.json", &atelier.racine, &dl)
            .await
            .is_err()
    );
}

/// Une réponse illisible ne remplace PAS une copie valide. Sans cette
/// précaution, un hôte qui sert une page d'erreur HTML en HTTP 200 écraserait
/// le dernier fil connu par du vide.
#[tokio::test]
async fn une_reponse_illisible_n_ecrase_pas_la_copie() {
    let atelier = Atelier::neuf("fil-html");
    let bonne = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    std::fs::write(atelier.racine.join("nouvelles.json"), &bonne).unwrap();

    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/nouvelles.json", "<html>maintenance</html>");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let _ = charger(
        &format!("{}/pack/samflix.json", serveur.base()),
        &atelier.racine,
        &dl,
    )
    .await;

    let apres = std::fs::read(atelier.racine.join("nouvelles.json")).unwrap();
    assert_eq!(apres, bonne.as_bytes(), "la copie valide a été écrasée");
}

// --- Les images ------------------------------------------------------------

/// Le type est déduit des OCTETS et non de l'extension : une extension vient
/// de l'URL, donc de l'hôte, donc de quelque chose qu'on ne contrôle pas au
/// même rythme que le code.
#[test]
fn le_type_se_deduit_des_octets() {
    assert_eq!(type_mime(b"\x89PNG\r\n\x1a\nreste"), Some("image/png"));
    assert_eq!(type_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(type_mime(b"GIF89a....."), Some("image/gif"));
    assert_eq!(type_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));

    assert_eq!(type_mime(b"<html>"), None);
    assert_eq!(type_mime(b""), None);
    // « RIFF » sans « WEBP » est un WAV : une image doit être une image.
    assert_eq!(type_mime(b"RIFF\0\0\0\0WAVEfmt "), None);
}

/// La borne existe parce que `Downloader::bytes` n'en a AUCUNE et accumule le
/// corps entier en mémoire, sous un délai de trois cents secondes. Un hôte
/// fautif servirait un fichier énorme, et le launcher grossirait jusqu'à se
/// faire tuer par le système — sans un message.
#[tokio::test]
async fn une_image_trop_grosse_est_refusee() {
    let atelier = Atelier::neuf("image-grosse");
    let serveur = mc_essais::Serveur::neuf().await;
    let mut enorme = b"\x89PNG\r\n\x1a\n".to_vec();
    enorme.resize(IMAGE_MAX + 1, 0);
    serveur.octets("/grosse.png", &enorme);
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let erreur = rapatrier(&serveur.url("/grosse.png"), &atelier.racine, &dl)
        .await
        .expect_err("une image de plus de 512 kio doit être refusée");
    assert!(format!("{erreur:#}").contains("512"), "{erreur:#}");
}

/// Ce qui n'est pas une image n'est pas mis en cache comme si c'en était une :
/// une page d'erreur HTML servie à la place d'une illustration deviendrait un
/// `data:text/html` chargé dans la fenêtre.
#[tokio::test]
async fn ce_qui_n_est_pas_une_image_est_refuse() {
    let atelier = Atelier::neuf("image-fausse");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/fausse.png", b"<html>404</html>");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    assert!(
        rapatrier(&serveur.url("/fausse.png"), &atelier.racine, &dl)
            .await
            .is_err()
    );
}

/// Le `data:` n'est fabriqué qu'À LA DEMANDE, depuis un fichier.
///
/// Garder les images en base64 dans le cache JSON ferait qu'un fil de huit
/// billets illustrés pèserait trois mégaoctets et demi, relus et resérialisés
/// à chaque ouverture de la page — pour un rendu qui n'en montre qu'une avant
/// qu'on ne fasse défiler.
#[tokio::test]
async fn l_image_est_un_fichier_et_le_data_url_est_fabrique_a_la_demande() {
    let atelier = Atelier::neuf("image-cache");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/i.png", b"\x89PNG\r\n\x1a\ncontenu");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let chemin = rapatrier(&serveur.url("/i.png"), &atelier.racine, &dl)
        .await
        .expect("l'image se rapatrie");

    assert!(chemin.is_file());
    // Un FICHIER, pas du base64 dans un JSON.
    assert_eq!(std::fs::read(&chemin).unwrap(), b"\x89PNG\r\n\x1a\ncontenu");

    let data = en_data_url(&chemin).expect("le data: se fabrique");
    assert!(data.starts_with("data:image/png;base64,"), "{data}");
}

/// Deux billets peuvent porter une `image.webp` chacun : le nom de cache est
/// l'empreinte de l'URL, sans quoi le second écraserait le premier.
#[tokio::test]
async fn deux_images_de_meme_nom_ne_se_marchent_pas_dessus() {
    let atelier = Atelier::neuf("image-collision");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/a/i.png", b"\x89PNG\r\n\x1a\nAAA");
    serveur.octets("/b/i.png", b"\x89PNG\r\n\x1a\nBBB");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let un = rapatrier(&serveur.url("/a/i.png"), &atelier.racine, &dl)
        .await
        .unwrap();
    let deux = rapatrier(&serveur.url("/b/i.png"), &atelier.racine, &dl)
        .await
        .unwrap();

    assert_ne!(un, deux);
    assert!(std::fs::read(&un).unwrap().ends_with(b"AAA"));
    assert!(std::fs::read(&deux).unwrap().ends_with(b"BBB"));
}

// --- Le base64, contre les vecteurs de la norme ----------------------------

/// Les sept vecteurs de la RFC 4648, section 10.
///
/// ## Pourquoi ils comptent plus qu'un test « ça a l'air d'un base64 »
///
/// Cet encodage est écrit à la main — trente lignes plutôt qu'une crate de
/// plus dans un binaire qu'on distribue. Le prix de ce choix est qu'aucune
/// bibliothèque éprouvée ne le couvre : c'est à nous de le faire.
///
/// Les sept vecteurs sont exactement conçus pour cela. Ils parcourent les
/// TROIS cas de remplissage — zéro, un et deux signes « = » — et assez de
/// motifs de bits pour qu'un décalage inversé, un « ou » devenu « et » ou un
/// masque déplacé d'un cran change la sortie. Sans eux, vingt mutations de ces
/// opérateurs survivaient : le code rendait quelque chose, et rien ne disait
/// que c'était le bon quelque chose.
#[test]
fn le_base64_suit_la_norme() {
    for (clair, attendu) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(clair.as_bytes()), attendu, "« {clair} »");
    }
}

/// Les octets hauts passent aussi : un encodage qui ne marcherait que sur de
/// l'ASCII serait inutile ici, où l'on encode des PNG et des WebP.
#[test]
fn le_base64_encode_les_octets_hauts() {
    assert_eq!(base64(&[0xFF, 0xFF, 0xFF]), "////");
    assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
    assert_eq!(base64(&[0xFB, 0xFF, 0xBF]), "+/+/");
    // Le début d'un PNG réel, pour que le test parle de ce qu'on encode.
    assert_eq!(base64(b"\x89PNG\r\n\x1a\n"), "iVBORw0KGgo=");
}

// --- Les bornes qui manquaient ---------------------------------------------

/// Douze octets SONT la signature WebP complète : ils suffisent.
///
/// Onze ne suffisent pas — il manquerait un octet à « WEBP ». Le test des deux
/// côtés de la borne est ce qui fixe l'opérateur : avec le seul cas « trop
/// court », `>` et `>=` rendraient la même chose.
#[test]
fn douze_octets_suffisent_a_reconnaitre_un_webp() {
    let douze = b"RIFF\0\0\0\0WEBP";
    assert_eq!(douze.len(), 12);
    assert_eq!(type_mime(douze), Some("image/webp"));
    assert_eq!(type_mime(&douze[..11]), None);
}

/// Un fil d'un autre schéma se lit, ET le dit.
///
/// Le journal seul ne suffisait pas : personne ne l'ouvre tant que rien n'a
/// l'air cassé, et c'est précisément le cas ici — les billets s'affichent très
/// bien. L'écart entre donc dans le compte rendu, que la page montre déjà.
#[test]
fn un_autre_schema_se_lit_et_le_dit() {
    let brut = format!(
        r#"{{"schema":99,"billets":[{}]}}"#,
        un_billet("a", "2026-09-01T00:00:00Z")
    );

    let fil = lire(brut.as_bytes(), FIL).expect("le fil se lit quand même");

    assert_eq!(fil.billets.len(), 1, "les billets ont été perdus");
    assert!(
        fil.ecartes.iter().any(|e| e.contains("schéma 99")),
        "le désaccord de schéma n'est dit nulle part : {:?}",
        fil.ecartes
    );
}

/// Et le bon schéma ne dit rien : un avertissement permanent finit par ne plus
/// être lu, et emporte avec lui ceux qui comptaient.
#[test]
fn le_bon_schema_ne_dit_rien() {
    let brut = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    let fil = lire(brut.as_bytes(), FIL).unwrap();
    assert!(fil.ecartes.is_empty(), "{:?}", fil.ecartes);
}

/// Une image d'EXACTEMENT la taille maximale est acceptée.
///
/// Le pendant du test qui refuse `IMAGE_MAX + 1`. Sans les deux, `>` et `>=`
/// se valent — et l'on refuserait une image pile à la limite sans que rien ne
/// l'ait décidé.
#[tokio::test]
async fn une_image_a_la_taille_maximale_est_acceptee() {
    let atelier = Atelier::neuf("image-pile");
    let serveur = mc_essais::Serveur::neuf().await;
    let mut pile = b"\x89PNG\r\n\x1a\n".to_vec();
    pile.resize(IMAGE_MAX, 0);
    serveur.octets("/pile.png", &pile);
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let chemin = rapatrier(&serveur.url("/pile.png"), &atelier.racine, &dl)
        .await
        .expect("une image pile à la limite doit passer");
    assert_eq!(
        std::fs::metadata(&chemin).unwrap().len() as usize,
        IMAGE_MAX
    );
}

/// Une image déjà en cache n'est pas redemandée. C'est ce qui fait que l'hôte
/// ne compte plus les ouvertures de la page — la seule chose que le
/// rapatriement achète vraiment côté vie privée.
#[tokio::test]
async fn une_image_deja_en_cache_n_est_pas_redemandee() {
    let atelier = Atelier::neuf("image-deja-la");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/i.png", b"\x89PNG\r\n\x1a\nX");
    let dl = mc_dl::Downloader::new("essai").unwrap();
    let url = serveur.url("/i.png");

    rapatrier(&url, &atelier.racine, &dl).await.unwrap();
    let apres_un = serveur.recues().len();
    rapatrier(&url, &atelier.racine, &dl).await.unwrap();

    assert_eq!(serveur.recues().len(), apres_un, "l'hôte a été redemandé");
}

/// **L'exemple de `docs/nouvelles.md` est lu par la suite, pas seulement par
/// les humains.**
///
/// Ce fichier est ce qu'on demande à mc-content d'implémenter : c'est donc un
/// contrat, servi à un autre dépôt. Un exemple de documentation qui ne
/// s'analyse pas est pire qu'une absence d'exemple — il fait perdre une
/// après-midi à qui le recopie, et personne ne pense à le mettre en doute.
///
/// Le test extrait le dernier bloc ```` ```json ```` du document. Le premier
/// est en ```` ```jsonc ````, avec des commentaires, et ne s'analyse pas — la
/// distinction des deux langages est délibérée et vaut d'être conservée.
#[test]
fn l_exemple_de_la_documentation_s_analyse() {
    let document = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/nouvelles.md"
    ))
    .expect("docs/nouvelles.md se lit depuis le crate");

    let exemple = document
        .rsplit_once("```json\n")
        .and_then(|(_, reste)| reste.split_once("\n```"))
        .map(|(bloc, _)| bloc)
        .expect("le document porte un bloc ```json");

    let fil = lire(exemple.as_bytes(), "https://exemple.invalid/nouvelles.json")
        .expect("l'exemple de la documentation s'analyse");

    assert!(
        fil.ecartes.is_empty(),
        "l'exemple ne doit rien faire écarter : {:?}",
        fil.ecartes
    );
    assert_eq!(fil.billets.len(), 2, "les deux billets sont retenus");
    // L'épinglé passe devant, quel que soit l'ordre du tableau.
    assert!(fil.billets[0].epinglee);
    // Et le corps est un arbre, jamais une chaîne : c'est la propriété à
    // laquelle le CSP a été desserré.
    assert!(!fil.billets[0].corps.is_empty());
}

/// **Un hôte sans fil ouvre une page vide, pas un message d'erreur.**
///
/// Le cas n'est pas théorique : au 18 septembre 2026, les trois hôtes rendent
/// 404 sur `nouvelles.json`. Le comportement d'avant affichait donc une erreur
/// à tous les joueurs, sur une page où il n'y avait simplement rien à dire.
///
/// Et le fil n'est PAS marqué hors ligne : l'hôte a répondu.
#[tokio::test]
async fn un_hote_sans_fil_rend_une_page_vide() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("sans-fil");
    let dl = mc_dl::Downloader::new("essai").expect("client");

    let fil = charger(&serveur.url("/pack/samflix.json"), &atelier.racine, &dl)
        .await
        .expect("un hôte sans fil n'est pas une erreur");

    assert!(fil.billets.is_empty());
    assert!(
        !fil.hors_ligne,
        "l'hôte a répondu : ce n'est pas du hors-ligne"
    );
    assert!(
        fil.ecartes.is_empty(),
        "rien n'a été écarté : il n'y avait rien"
    );
}

/// Et une copie en cache ne ressuscite PAS un fil que l'hôte a retiré.
///
/// C'est la moitié du choix qui demande à être défendue : on pourrait servir
/// l'ancien fil. Mais un billet retiré de l'hôte l'a été par quelqu'un, et le
/// réafficher parce qu'on en garde une copie serait désobéir à ce geste.
#[tokio::test]
async fn un_fil_retire_ne_revient_pas_du_cache() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("fil-retire");
    let dl = mc_dl::Downloader::new("essai").expect("client");

    // Une copie qui traîne, d'un fil qui existait hier.
    std::fs::write(
        atelier.racine.join("nouvelles.json"),
        br#"{"schema":1,"billets":[{"id":"vieux","titre":"Retire","date":"2026-09-01T00:00:00Z","corps":"."}]}"#,
    )
    .expect("copie");

    let fil = charger(&serveur.url("/pack/samflix.json"), &atelier.racine, &dl)
        .await
        .expect("un hôte sans fil n'est pas une erreur");

    assert!(
        fil.billets.is_empty(),
        "le fil retiré est revenu du cache : {:?}",
        fil.billets
    );
}
