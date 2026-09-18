//! The packaged build's acceptance test, played by the window itself.
//!
//! ## What used to be checked by eye, and why that's not enough
//!
//! Seven points decide whether a packaged build renders what `ng serve`
//! shows, and **none of them show up in development**: it's only in
//! production that Tauri serves the page, rewrites the CSP and applies
//! dynamic `import()`s.
//!
//! The acceptance test used to be a checklist to run through in the
//! inspector, one point after another. A checklist gets run through once —
//! the day it was written — then never played again, and what it protected
//! breaks silently three months later.
//!
//! This module plays it on every launch of a development build, and logs it
//! in one line. What's missing is named; what's fine isn't chatty.
//!
//! ## What's left to a human
//!
//! This module says that `backdrop-filter` **is computed**, not that it's
//! **pretty**; that decorations are absent, not that the window's shadow is
//! correct under GNOME. Everything that comes down to taste or to the
//! window manager stays in `docs/interface.md` — but the list there is now
//! short, and every point that leaves it is a point that won't be asked
//! about again.

use serde::Deserialize;

/// The name of the event the window reports through.
#[cfg(debug_assertions)]
const EVENT: &str = "acceptance";

/// What the window observed about itself.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Report {
    /// The value of `--gold` on `:root`, or empty.
    ///
    /// **A design system token, not the `data-theme` attribute.** Setting
    /// the attribute proves nothing — it's hardcoded in `index.html` and
    /// would be there even if `tokens.css` weren't served. What matters is
    /// checking that the tokens actually made it into the document.
    ///
    /// `--gold` rather than another one: it's the design system's single
    /// accent, and everything that catches the eye in the window depends on
    /// it.
    pub theme: String,
    /// The COMPUTED value of a glass surface's `backdrop-filter`.
    ///
    /// This proves the prefix is the one WebKitGTK understands: without it,
    /// every glass surface in the launcher renders flat, and nothing in the
    /// console says so.
    pub glass: String,
    /// A property that exists ONLY in a component's `styleUrl`.
    ///
    /// This is the test for `style-src` being loosened. It used to bear on
    /// the title bar's `backdrop-filter` — which no longer holds, since the
    /// glass now comes from the design system's global stylesheet rather
    /// than from a component style. A custom property declared in `app.css`
    /// and nowhere else tells the two apart unambiguously: it can only
    /// arrive through a component style.
    pub component_style: String,
    /// A color passed through `color-mix()`, as the engine renders it.
    pub color_mix: String,
    /// Is a ROUTE component mounted?
    ///
    /// This proves a `loadComponent` resolved its `import()`, so that
    /// `script-src` lets lazy fragments through — the one point in the
    /// acceptance test that used to be asserted without ever being
    /// demonstrated.
    pub route_mounted: bool,
    /// How many stylesheets the document carries.
    pub sheets: usize,
    /// The CSP violations recorded since the very first script.
    pub violations: Vec<String>,
}

/// What's missing, named.
///
/// Returns an empty list when everything's fine. This is the PURE half of
/// the module: it can be re-read in a test, without a display server, and
/// it's the one that carries the thresholds.
pub fn gaps(report: &Report) -> Vec<String> {
    let mut gaps = Vec::new();

    if report.theme.is_empty() {
        gaps.push(
            "--gold isn't set on :root: the design system's tokens aren't being served".into(),
        );
    }
    if report.component_style.is_empty() {
        gaps.push(
            "app.css's marker property doesn't arrive: a component style is being \
             rejected, so `style-src` doesn't carry 'unsafe-inline'"
                .into(),
        );
    }
    // The computed value of an unrecognized property is either the empty
    // string or "none"; both say the same thing here — the glass is flat.
    if report.glass.is_empty() || report.glass == "none" {
        gaps.push(
            "a glass surface's backdrop-filter renders nothing: \
             the -webkit- prefix is missing, and all the frosted glass renders flat"
                .into(),
        );
    }
    if !report.color_mix.starts_with("rgb") && !report.color_mix.starts_with("oklab") {
        gaps.push(format!(
            "color-mix() isn't resolved by the engine: \"{}\"",
            report.color_mix
        ));
    }
    if report.sheets == 0 {
        gaps.push("no stylesheet: the CSS isn't served through <link>".into());
    }
    if !report.route_mounted {
        gaps.push(
            "no route component mounted: the lazy fragment didn't arrive, \
             which `script-src` can break without a word"
                .into(),
        );
    }
    for violation in &report.violations {
        gaps.push(format!("CSP violation: {violation}"));
    }

    gaps
}

/// The script injected BEFORE the document.
///
/// It observes nothing: it only sets up the violation collector. It's the
/// only way to see them all — a violation occurs when the page loads, i.e.
/// before anything we could run afterwards.
#[cfg(debug_assertions)]
const COLLECTOR: &str = "window.__violations=[];\
document.addEventListener('securitypolicyviolation',\
e=>window.__violations.push(e.violatedDirective+' ← '+(e.blockedURI||'inline')));";

/// The plugin that sets up the collector.
///
/// As a plugin, because that's the only way to run a script before the
/// document on a webview declared in `tauri.conf.json`.
///
/// It exists in both builds, and only sets up its script in development:
/// the `cfg` is INSIDE rather than on the function, which spares the window
/// mount from having two shapes depending on the profile — a divergence
/// whose symptom would be "it works in debug".
pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    let builder = tauri::plugin::Builder::new("acceptance");
    #[cfg(debug_assertions)]
    let builder = builder.js_init_script(COLLECTOR.to_string());
    builder.build()
}

/// What the window observes about itself, once the page has rendered.
///
/// **It WAITS for a route component to be mounted**, up to eight seconds.
/// Without this wait, the probe observes during boot — which takes nine
/// hundred milliseconds, the time of the network handshake — and concludes
/// the lazy fragment never arrived. That's what it did on the first try, on
/// a perfectly sound build.
///
/// The eight-second delay isn't a nicety: past that, it's no longer a wait,
/// it's an outage, and the report has to say so rather than hang.
#[cfg(debug_assertions)]
const OBSERVATION: &str = "(async()=>{\
const R='app-spawn,app-signin,app-news,app-settings';\
for(let i=0;i<80&&!document.querySelector(R);i++)\
await new Promise(r=>setTimeout(r,100));\
const v=document.querySelector('.hm-glass,.hm-nav,.hm-dialog,.hm-player,.hm-toast');\
const s=v?getComputedStyle(v):null;\
const t=document.createElement('div');\
t.style.color='color-mix(in oklab, red 50%, blue)';\
document.body.appendChild(t);\
const cm=getComputedStyle(t).color;\
t.remove();\
const root=getComputedStyle(document.documentElement);\
return{theme:root.getPropertyValue('--gold').trim(),\
componentStyle:getComputedStyle(document.querySelector('app-root'))\
.getPropertyValue('--acceptance-component-style').trim(),\
glass:s?(s.backdropFilter||s.webkitBackdropFilter||''):'',\
colorMix:cm,\
routeMounted:!!document.querySelector(R),\
sheets:document.styleSheets.length,\
violations:window.__violations||[]};})()";

/// Asks the window for the acceptance report, and logs what's missing.
///
/// Called from `front_ready`, after the first render: before that, the
/// displayed route hasn't loaded its fragment yet, and the one point that
/// tests `import()` would give a false negative.
#[cfg(debug_assertions)]
pub fn probe(app: &tauri::AppHandle) {
    use tauri::{Listener, Manager};

    app.once(EVENT, |event| {
        let report: Report = serde_json::from_str(event.payload()).unwrap_or_default();
        let gaps = gaps(&report);
        if gaps.is_empty() {
            tracing::info!(
                theme = report.theme,
                glass = report.glass,
                route_mounted = report.route_mounted,
                sheets = report.sheets,
                "build acceptance: all eight points pass"
            );
            return;
        }
        for gap in gaps {
            tracing::error!(gap, "build acceptance");
        }
    });

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let script = format!(
        "Promise.resolve({OBSERVATION}).then(p=>window.__TAURI_INTERNALS__.invoke(\
         'plugin:event|emit',{{event:'{EVENT}',payload:p}}))"
    );
    if let Err(error) = window.eval(&script) {
        tracing::warn!(error = %error, "acceptance not run");
    }
}

#[cfg(test)]
#[path = "acceptance.test.rs"]
mod tests;
