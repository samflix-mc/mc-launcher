use super::{Verdict, judge};

/// The string from `tauri.conf.json`, as is.
const CONFIGURED: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://mc-heads.net; connect-src 'self' ipc: http://ipc.localhost";

/// **The string actually served**, captured by the probe on September 18,
/// 2026 on a `tauri build --debug`, and copied verbatim.
///
/// It corrects a premise: Tauri does NOT set a nonce on `script-src`, it
/// sets the DIGESTS of the scripts it injects itself into the page
/// (`manager/mod.rs:64`, `Assets::csp_hashes`). Eight of them, none from the
/// front end: `web/dist/launcher/browser/index.html` carries only a single
/// `<script src=…>` and no `<style>` tag.
///
/// What this changes for us: nothing, and that's the point. `style-src`
/// comes out intact, `'unsafe-inline'` holds, component styles pass. But the
/// reasoning that predicted it talked about a nonce, and a reasoning that's
/// right for the wrong reason turns on you at the next version change.
const SERVED: &str = "style-src 'self' 'unsafe-inline'; img-src 'self' data: https://mc-heads.net; default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; script-src 'self' 'sha256-raN9g3H4/dO8mZTghxmpwIgmmDWT4PrwtdQGtErN334=' 'sha256-b2pRmdZzRK2UEE2Y2izHHGpcCBmGUI3ajk8odRJT+jI=' 'sha256-WoWckt9X/P4/opegC4iuUy1+J6pfCaXJ2CkVE2gaiMA=' 'sha256-AChnpMhGIxxHNYcAGoNFqmgzgXq0sEBJDR5+ReXoCaY=' 'sha256-4YLDHlNhJYuS+BrRUUkXea3SkKzwHTQ80BebtmRWptQ=' 'sha256-ga43UjNWBepYSeJXXGAzXPmqy/vdbnKX/0pqlgdMjp4=' 'sha256-L22/pkuIINLKMKP4klrbibY7UEdR92P5HuyBYzhuy1I=' 'sha256-6Uv340HkYRGMuqMHMPg9OCr/36YC36+i5P8h/e9lL7A='";

/// The configured string already carries the loosening we expect.
///
/// This test doesn't exercise Tauri: it exercises that we know how to read
/// it, and it's the reference the next one compares against.
#[test]
fn the_configured_string_lets_component_styles_pass() {
    assert_eq!(
        judge(CONFIGURED),
        Some(Verdict {
            component_styles_pass: true,
            nonce_on_style: false,
            strict_scripts: true,
        })
    );
}

/// The nominal case, as it was MEASURED and not as it was predicted.
///
/// If the probe ever logs something other than this verdict, it's the
/// reasoning in `tauri.conf.json` that needs revisiting — not this test.
#[test]
fn what_is_actually_served_leaves_the_styles_alone() {
    assert_eq!(
        judge(SERVED),
        Some(Verdict {
            component_styles_pass: true,
            nonce_on_style: false,
            strict_scripts: true,
        })
    );
}

/// **The failure this probe exists to catch.**
///
/// A nonce on `style-src` cancels `'unsafe-inline'` — level 3 rule — and
/// every component style drops. Nothing crashes: the window opens without
/// styling, and in a packaged build only.
#[test]
fn a_nonce_on_styles_cancels_the_loosening() {
    let served = "default-src 'self'; script-src 'self' 'nonce-a'; style-src 'self' 'unsafe-inline' 'nonce-a'";
    assert_eq!(
        judge(served),
        Some(Verdict {
            component_styles_pass: false,
            nonce_on_style: true,
            strict_scripts: true,
        })
    );
}

/// `'unsafe-eval'` on scripts is never strict, nonce or not.
#[test]
fn unsafe_eval_disarms_the_scripts() {
    let served = "style-src 'self' 'unsafe-inline'; script-src 'self' 'nonce-a' 'unsafe-eval'";
    let verdict = judge(served).expect("style-src is there");
    assert!(!verdict.strict_scripts);
}

/// `'unsafe-inline'` on scripts WITHOUT a nonce isn't strict either.
#[test]
fn a_script_inline_without_a_nonce_is_not_strict() {
    let served = "style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'";
    let verdict = judge(served).expect("style-src is there");
    assert!(!verdict.strict_scripts);
}

/// Without `style-src`, there's no verdict — and certainly not a favorable
/// one by default.
#[test]
fn without_style_src_there_is_no_answer() {
    assert_eq!(judge("default-src 'self'; script-src 'self'"), None);
}

/// `script-src` is a PREFIX of `script-src-elem`: judging the wrong
/// directive would give a right answer to a question that isn't asked.
#[test]
fn a_neighboring_directive_is_not_confused() {
    let served =
        "style-src 'self' 'unsafe-inline'; script-src-elem 'unsafe-eval'; script-src 'self'";
    let verdict = judge(served).expect("style-src is there");
    assert!(
        verdict.strict_scripts,
        "it was script-src-elem carrying the 'unsafe-eval', not script-src"
    );
}

/// An empty directive means "no source", and reads without panicking.
#[test]
fn an_empty_directive_reads_fine() {
    let verdict = judge("style-src; script-src 'self'").expect("style-src is there");
    assert!(!verdict.component_styles_pass);
    assert!(!verdict.nonce_on_style);
}

/// Header spacing isn't normalized by browsers, and it isn't by servers
/// either: reading must survive it.
#[test]
fn spacing_does_not_change_the_verdict() {
    let tight = "default-src 'self';script-src 'self';style-src 'self' 'unsafe-inline'";
    let loose =
        "  default-src 'self' ;  script-src   'self' ;  style-src   'self'   'unsafe-inline'  ";
    assert_eq!(judge(tight), judge(loose));
    assert!(
        judge(tight)
            .expect("style-src is there")
            .component_styles_pass
    );
}

/// Digests are not a nonce, and the verdict must not confuse them: a
/// `'sha256-…'` on `script-src` covers an inline script by name, without
/// loosening anything for the others.
#[test]
fn digests_are_not_a_nonce() {
    let verdict = judge(SERVED).expect("style-src is there");
    assert!(!verdict.nonce_on_style);
    assert!(verdict.strict_scripts);
    assert!(
        !SERVED.contains("'nonce-"),
        "the captured string contains no nonce: that's the fact this module documents"
    );
}
