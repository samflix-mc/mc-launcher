use super::{acceptable, image_absolue, meme_origine};

const FIL: &str = "https://mc-launcher.ggy.info/pack/nouvelles.json";

/// LE test du module, et il se démontre plutôt qu'il ne se raisonne :
/// `https://mc-launcher.ggy.info.evil.example/` COMMENCE bien par
/// `https://mc-launcher.ggy.info`. Une comparaison par préfixe l'accepterait.
#[test]
fn un_hote_qui_commence_par_le_bon_nom_n_est_pas_le_bon() {
    for imposteur in [
        "https://mc-launcher.ggy.info.evil.example/i.webp",
        "https://mc-launcher.ggy.infoo/i.webp",
        "https://evil.example/mc-launcher.ggy.info/i.webp",
    ] {
        assert!(
            !meme_origine(imposteur, FIL),
            "accepté à tort : {imposteur}"
        );
    }
}

/// Un `user@host` : ce qui compte est APRÈS l'arobase. Sans ce découpage,
/// `https://mc-launcher.ggy.info@evil.example/` passerait pour notre hôte,
/// parce que le début de l'autorité ressemble au bon nom.
#[test]
fn une_autorite_avec_arobase_ne_trompe_pas() {
    assert!(!meme_origine(
        "https://mc-launcher.ggy.info@evil.example/i.webp",
        FIL
    ));
}

#[test]
fn le_meme_hote_passe() {
    assert!(meme_origine("https://mc-launcher.ggy.info/a/b.webp", FIL));
    // La casse de l'hôte ne compte pas : les DNS ne la distinguent pas.
    assert!(meme_origine("https://MC-Launcher.GGY.info/b.webp", FIL));
}

/// Le schéma compte aussi : servir une image en clair depuis un fil en HTTPS
/// ferait apparaître un avertissement de contenu mixte, et surtout ouvrirait
/// une porte qu'on vient de fermer.
#[test]
fn un_autre_schema_ne_passe_pas() {
    assert!(!meme_origine("http://mc-launcher.ggy.info/i.webp", FIL));
}

/// Et le schéma n'est PAS figé à `https:` : le serveur d'essai ne sert que du
/// `http://127.0.0.1:<port>`, et figer https rendrait toute la moitié réseau
/// de ce crate intestable — un module qu'on ne peut pas éprouver finit par
/// contenir ce qu'on n'a pas voulu.
#[test]
fn un_fil_local_en_clair_reste_coherent_avec_lui_meme() {
    let local = "http://127.0.0.1:8080/pack/nouvelles.json";
    assert!(meme_origine("http://127.0.0.1:8080/i.webp", local));
    assert!(!meme_origine("http://127.0.0.1:9090/i.webp", local));
}

// --- Les images ------------------------------------------------------------

#[test]
fn une_image_relative_se_resout_contre_le_fil() {
    assert_eq!(
        image_absolue("saison3.webp", FIL).as_deref(),
        Some("https://mc-launcher.ggy.info/pack/saison3.webp")
    );
    assert_eq!(
        image_absolue("/saison3.webp", FIL).as_deref(),
        Some("https://mc-launcher.ggy.info/pack/saison3.webp")
    );
}

/// Un `..` remonterait hors du répertoire publié. On refuse plutôt que de
/// normaliser : rien de légitime n'en a besoin, et normaliser demanderait de
/// reproduire les règles de résolution d'URL, qui ont leurs propres pièges.
#[test]
fn une_image_qui_remonte_est_refusee() {
    assert_eq!(image_absolue("../secret/i.webp", FIL), None);
    assert_eq!(image_absolue("a/../../i.webp", FIL), None);
}

#[test]
fn une_image_absolue_d_ailleurs_est_refusee() {
    assert_eq!(image_absolue("https://evil.example/i.webp", FIL), None);
    assert_eq!(
        image_absolue("https://mc-launcher.ggy.info/i.webp", FIL).as_deref(),
        Some("https://mc-launcher.ggy.info/i.webp")
    );
}

// --- Les liens -------------------------------------------------------------

/// Un lien a le droit de sortir : c'est le propre d'un lien, et il n'est
/// jamais suivi DANS la fenêtre — le front le confie au navigateur du système.
#[test]
fn un_lien_vers_l_exterieur_est_accepte() {
    for href in [
        "https://modrinth.com/mod/jei",
        "http://exemple.invalid/",
        "mailto:contact@exemple.invalid",
    ] {
        assert!(acceptable(href, FIL).is_some(), "refusé à tort : {href}");
    }
}

/// Ce qui n'a aucun sens dans un billet, et tout à gagner à être refusé.
#[test]
fn les_schemas_dangereux_sont_refuses() {
    for href in [
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "vbscript:msgbox",
        "/chemin/relatif",
        "",
    ] {
        assert!(acceptable(href, FIL).is_none(), "accepté à tort : {href}");
    }
}

/// La comparaison est insensible à la casse : `JavaScript:` passerait sinon
/// entre les mailles, et c'est le contournement le plus ancien qui soit.
#[test]
fn la_casse_du_schema_ne_sauve_pas_un_lien_dangereux() {
    for href in ["JavaScript:alert(1)", "JAVASCRIPT:alert(1)", "DaTa:x"] {
        assert!(acceptable(href, FIL).is_none(), "accepté à tort : {href}");
    }
}

/// Les espaces autour ne doivent pas suffire à contourner le contrôle.
#[test]
fn les_espaces_ne_sauvent_pas_un_lien_dangereux() {
    assert!(acceptable("  javascript:alert(1)  ", FIL).is_none());
    assert!(acceptable("  https://exemple.invalid/  ", FIL).is_some());
}
