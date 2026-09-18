use super::{Verdict, juger};

/// La chaîne de `tauri.conf.json`, telle quelle.
const CONFIGUREE: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://mc-heads.net; connect-src 'self' ipc: http://ipc.localhost";

/// **La chaîne réellement servie**, relevée par la sonde le 18 septembre 2026
/// sur un `tauri build --debug`, et recopiée telle quelle.
///
/// Elle corrige une prémisse : Tauri ne pose PAS de nonce sur `script-src`,
/// il pose les EMPREINTES des scripts qu'il injecte lui-même dans la page
/// (`manager/mod.rs:64`, `Assets::csp_hashes`). Huit, dont aucune ne vient
/// du front : `web/dist/launcher/browser/index.html` ne porte qu'un seul
/// `<script src=…>` et pas une balise `<style>`.
///
/// Ce que cela change pour nous : rien, et c'est le point. `style-src` sort
/// intact, `'unsafe-inline'` tient, les styles de composant passent. Mais le
/// raisonnement qui le prédisait parlait d'un nonce, et un raisonnement juste
/// pour une mauvaise raison se retourne au premier changement de version.
const SERVIE: &str = "style-src 'self' 'unsafe-inline'; img-src 'self' data: https://mc-heads.net; default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; script-src 'self' 'sha256-raN9g3H4/dO8mZTghxmpwIgmmDWT4PrwtdQGtErN334=' 'sha256-b2pRmdZzRK2UEE2Y2izHHGpcCBmGUI3ajk8odRJT+jI=' 'sha256-WoWckt9X/P4/opegC4iuUy1+J6pfCaXJ2CkVE2gaiMA=' 'sha256-AChnpMhGIxxHNYcAGoNFqmgzgXq0sEBJDR5+ReXoCaY=' 'sha256-4YLDHlNhJYuS+BrRUUkXea3SkKzwHTQ80BebtmRWptQ=' 'sha256-ga43UjNWBepYSeJXXGAzXPmqy/vdbnKX/0pqlgdMjp4=' 'sha256-L22/pkuIINLKMKP4klrbibY7UEdR92P5HuyBYzhuy1I=' 'sha256-6Uv340HkYRGMuqMHMPg9OCr/36YC36+i5P8h/e9lL7A='";

/// La chaîne configurée porte déjà le desserrage qu'on attend.
///
/// Ce test n'éprouve pas Tauri : il éprouve qu'on sait lire, et c'est la
/// référence à laquelle la suivante se compare.
#[test]
fn la_chaine_configuree_laisse_passer_les_styles_de_composant() {
    assert_eq!(
        juger(CONFIGUREE),
        Some(Verdict {
            styles_de_composant_passent: true,
            nonce_sur_style: false,
            scripts_stricts: true,
        })
    );
}

/// Le cas nominal, tel qu'il a été MESURÉ et non tel qu'il était prédit.
///
/// Si la sonde journalise un jour autre chose que ce verdict-là, c'est le
/// raisonnement de `tauri.conf.json` qui est à revoir — pas ce test.
#[test]
fn ce_qui_est_reellement_servi_laisse_les_styles_tranquilles() {
    assert_eq!(
        juger(SERVIE),
        Some(Verdict {
            styles_de_composant_passent: true,
            nonce_sur_style: false,
            scripts_stricts: true,
        })
    );
}

/// **La panne que la sonde existe pour attraper.**
///
/// Un nonce sur `style-src` annule `'unsafe-inline'` — règle du niveau 3 — et
/// tous les styles de composant tombent. Rien ne plante : la fenêtre s'ouvre
/// sans mise en forme, et en build empaqueté seulement.
#[test]
fn un_nonce_sur_les_styles_annule_le_desserrage() {
    let servie = "default-src 'self'; script-src 'self' 'nonce-a'; style-src 'self' 'unsafe-inline' 'nonce-a'";
    assert_eq!(
        juger(servie),
        Some(Verdict {
            styles_de_composant_passent: false,
            nonce_sur_style: true,
            scripts_stricts: true,
        })
    );
}

/// `'unsafe-eval'` sur les scripts n'est jamais strict, nonce ou pas.
#[test]
fn unsafe_eval_desarme_les_scripts() {
    let servie = "style-src 'self' 'unsafe-inline'; script-src 'self' 'nonce-a' 'unsafe-eval'";
    let verdict = juger(servie).expect("style-src est là");
    assert!(!verdict.scripts_stricts);
}

/// `'unsafe-inline'` sur les scripts SANS nonce n'est pas strict non plus.
#[test]
fn un_inline_de_script_sans_nonce_n_est_pas_strict() {
    let servie = "style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'";
    let verdict = juger(servie).expect("style-src est là");
    assert!(!verdict.scripts_stricts);
}

/// Sans `style-src`, il n'y a pas de verdict — et surtout pas un verdict
/// favorable par défaut.
#[test]
fn sans_style_src_il_n_y_a_pas_de_reponse() {
    assert_eq!(juger("default-src 'self'; script-src 'self'"), None);
}

/// `script-src` est un PRÉFIXE de `script-src-elem` : juger la mauvaise
/// directive donnerait une réponse juste à une question qu'on ne pose pas.
#[test]
fn une_directive_voisine_n_est_pas_confondue() {
    let servie =
        "style-src 'self' 'unsafe-inline'; script-src-elem 'unsafe-eval'; script-src 'self'";
    let verdict = juger(servie).expect("style-src est là");
    assert!(
        verdict.scripts_stricts,
        "c'est script-src-elem qui portait le 'unsafe-eval', pas script-src"
    );
}

/// Une directive vide vaut « aucune source », et se lit sans paniquer.
#[test]
fn une_directive_vide_se_lit() {
    let verdict = juger("style-src; script-src 'self'").expect("style-src est là");
    assert!(!verdict.styles_de_composant_passent);
    assert!(!verdict.nonce_sur_style);
}

/// L'espacement de l'en-tête n'est pas normalisé par les navigateurs, et il
/// ne l'est pas non plus par les serveurs : la lecture doit y survivre.
#[test]
fn l_espacement_ne_change_pas_le_verdict() {
    let serre = "default-src 'self';script-src 'self';style-src 'self' 'unsafe-inline'";
    let large =
        "  default-src 'self' ;  script-src   'self' ;  style-src   'self'   'unsafe-inline'  ";
    assert_eq!(juger(serre), juger(large));
    assert!(
        juger(serre)
            .expect("style-src est là")
            .styles_de_composant_passent
    );
}

/// Des EMPREINTES ne sont pas un nonce, et le verdict ne doit pas les
/// confondre : un `'sha256-…'` sur `script-src` couvre un script inline
/// nommément, sans rien desserrer pour les autres.
#[test]
fn des_empreintes_ne_sont_pas_un_nonce() {
    let verdict = juger(SERVIE).expect("style-src est là");
    assert!(!verdict.nonce_sur_style);
    assert!(verdict.scripts_stricts);
    assert!(
        !SERVIE.contains("'nonce-"),
        "la chaîne relevée ne contient aucun nonce : c'est le fait que ce module documente"
    );
}
