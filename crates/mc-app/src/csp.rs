//! What the SERVED CSP says — read from the window, not from the config.
//!
//! ## Why the config isn't enough to answer
//!
//! `tauri.conf.json` carries a string; what the window receives is another
//! one. Between the two, Tauri rewrites it: it adds to `script-src` the
//! DIGESTS of the scripts it injects itself (`manager/mod.rs:64`), and it
//! would set a nonce on `style-src` if it found a `<style>` tag in the
//! embedded HTML (`tauri-utils/src/html.rs:154-157`). But a nonce CANCELS
//! `'unsafe-inline'` — that's the level 3 rule — and the loosening that every
//! component style relies on would then drop without a word, **in a packaged
//! build only**.
//!
//! That's exactly the kind of drift you don't discover: the window doesn't
//! crash, it just renders without styling.
//!
//! ## Why a probe rather than a glance at the inspector
//!
//! The inspector only exists in a development build, it requires a human
//! action, and it only answers for the build at hand that day. A probe that
//! logs answers on every launch, and the day someone adds a `<style>` in
//! `index.html` the line changes on its own.
//!
//! The half that matters — [`judge`] — is a PURE function: it's testable
//! without a display server.

/// The name of the event by which the window reports its header.
#[cfg(debug_assertions)]
const EVENT: &str = "csp-served";

/// What reading the header lets us conclude.
#[cfg(any(debug_assertions, test))]
#[derive(Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Do component styles pass: does `style-src` admit inline, with nothing
    /// canceling it?
    pub component_styles_pass: bool,
    /// Was a nonce set on `style-src` — which neutralizes `'unsafe-inline'`,
    /// and therefore every `styleUrl` in the front end.
    pub nonce_on_style: bool,
    /// `script-src` admits neither `eval` nor inline uncovered by a nonce:
    /// it's what protects the privileged origin where `invoke` is reachable.
    pub strict_scripts: bool,
}

/// Judges a `Content-Security-Policy` header.
///
/// Returns `None` if the `style-src` directive is absent: that's not a
/// default verdict but an unanswered question, and confusing it with
/// "everything's fine" would be the only way to miss what the probe is
/// looking for.
#[cfg(any(debug_assertions, test))]
pub fn judge(header: &str) -> Option<Verdict> {
    let style = directive(header, "style-src")?;
    let script = directive(header, "script-src").unwrap_or_default();

    // A nonce CANCELS `'unsafe-inline'` (CSP level 3): the two present
    // together read as the nonce alone, and that's the entire subtlety this
    // probe exists to settle.
    let nonce_on_style = style.contains("'nonce-");
    let nonce_on_script = script.contains("'nonce-");

    Some(Verdict {
        component_styles_pass: style.contains("'unsafe-inline'") && !nonce_on_style,
        nonce_on_style,
        strict_scripts: !script.contains("'unsafe-eval'")
            && (!script.contains("'unsafe-inline'") || nonce_on_script),
    })
}

/// The content of a directive, or nothing.
///
/// Compares on the whole NAME and not on a prefix: `script-src` is a prefix
/// of `script-src-elem`, and confusing the two would judge the wrong
/// directive — precisely the one that isn't the one being loosened.
#[cfg(any(debug_assertions, test))]
fn directive<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').map(str::trim).find_map(|piece| {
        let rest = piece.strip_prefix(name)?;
        // The name must be followed by a space — or nothing, for an empty
        // directive, which exists and means "no source".
        match rest.chars().next() {
            None => Some(""),
            Some(' ') => Some(rest.trim_start()),
            Some(_) => None,
        }
    })
}

/// Asks the window for the header it actually received, and logs it.
///
/// Called from `front_ready`, and nowhere else: it's the only moment where
/// the page is certainly loaded, so the only one where an `eval` reaches its
/// target.
///
/// Absent from the production binary. Not that the line would be dangerous
/// there — it says nothing a `curl` on the bundle wouldn't say — but because
/// an internal consistency check should cost a player nothing.
#[cfg(debug_assertions)]
pub fn probe(app: &tauri::AppHandle) {
    use tauri::{Listener, Manager};

    app.once(EVENT, |event| {
        let header: String = serde_json::from_str(event.payload()).unwrap_or_default();
        match judge(&header) {
            Some(verdict) => tracing::info!(
                header,
                component_styles_pass = verdict.component_styles_pass,
                nonce_on_style = verdict.nonce_on_style,
                strict_scripts = verdict.strict_scripts,
                "CSP served to the window"
            ),
            None => tracing::warn!(header, "CSP served without a style-src directive"),
        }
    });

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    // `location.href` and not a hardcoded path: the origin changes with the
    // platform — `tauri://localhost` on Linux and macOS,
    // `http://tauri.localhost` on Windows — and a fixed path would only
    // report the header on one of the three.
    if let Err(error) = window.eval(concat!(
        "fetch(location.href).then(r=>window.__TAURI_INTERNALS__.invoke(",
        "'plugin:event|emit',{event:'csp-served',",
        "payload:r.headers.get('content-security-policy')||''}))",
    )) {
        tracing::warn!(error = %error, "CSP probe not executed");
    }
}

#[cfg(test)]
#[path = "csp.test.rs"]
mod tests;
