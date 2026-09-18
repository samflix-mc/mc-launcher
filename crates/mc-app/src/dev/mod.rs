//! The launcher without its window, for working on the interface.
//!
//! ## The problem this module solves
//!
//! The front can only be looked at inside the Tauri window, because that's
//! where `invoke` answers. But in that window there's no hot reload, no
//! inspector in production, and no way to put the screen in a chosen state.
//! Every try costs a full build, and the rare states — three missing mods,
//! an install stuck halfway — can't be triggered.
//!
//! This module serves the SAME commands over HTTP. The front then runs in an
//! ordinary browser, with its hot reload, its dev tools, and a state you
//! pick with one request.
//!
//! ## What guarantees it doesn't ship to production
//!
//! It's behind the `dev-server` feature, which isn't enabled by default,
//! and it's only compiled into a separate binary — `mc-dev-server`, declared
//! with `required-features`. `cargo tauri build` doesn't enable it: the code
//! isn't "disabled" in the launcher's binary, it isn't there.
//!
//! ## What's real and what's simulated
//!
//! Real: `brand`, `path`, `settings`, `save_settings`. They touch neither
//! the network nor the game's disk, and watching them lie wouldn't teach
//! anything — `save_settings` therefore really writes, which is exactly
//! what we want to put to the test, since that's where the bounds apply.
//!
//! Simulated: everything that authenticates, installs or launches. See
//! [`scenario`].

pub mod http;
pub mod scenario;

use std::sync::{Arc, Mutex};

use serde_json::json;
use tokio::sync::broadcast;

use http::{Request, Response};
use scenario::State;

/// The default port.
///
/// 1421 because Angular's server takes 1420: the two run together, and
/// seeing the two numbers follow each other avoids having to remember which
/// is which.
pub const PORT: u16 = 1421;

/// What the server keeps between two requests.
pub struct Context {
    pub state: Mutex<State>,
    pub events: broadcast::Sender<String>,
}

impl Context {
    pub fn new(start: State) -> Arc<Context> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Context {
            state: Mutex::new(start),
            events,
        })
    }

    fn read(&self) -> State {
        *self.state.lock().expect("state not poisoned")
    }

    /// Publishes an event, in the shape the front expects.
    ///
    /// The name travels alongside the payload: SSE has only one stream,
    /// where `listen()` has one per name. The front demultiplexes.
    fn emit(&self, name: &str, payload: serde_json::Value) {
        let message = json!({ "event": name, "payload": payload }).to_string();
        // An error here only means nobody's listening.
        let _ = self.events.send(message);
    }
}

/// The routing, separated from the transport.
///
/// An ordinary function that takes a request and renders a response: that's
/// what makes it testable without opening a socket, and it's where the
/// module's only decisions live.
pub async fn router(context: Arc<Context>, request: Request) -> Response {
    let state = context.read();

    // The scenario changes in one request — that's the whole point.
    if let Some(name) = request.path.strip_prefix("/scenario/") {
        return match State::from_name(name) {
            Some(new_state) => {
                *context.state.lock().expect("state not poisoned") = new_state;
                context.emit(
                    crate::cinematic::EVENT_PROGRESS,
                    serde_json::to_value(new_state.progress()).unwrap_or(serde_json::Value::Null),
                );
                Response::json(json!({ "scenario": name }).to_string())
            }
            None => Response::error(404, &format!("unknown scenario: {name}")),
        };
    }

    let Some(name) = request.path.strip_prefix("/command/") else {
        return home(state);
    };

    match name {
        // --- What calls the real implementation -------------------------
        "brand" => value(&crate::commands::brand()),
        "path" => value(&scenario::path()),
        "settings" => value(&crate::commands::settings::settings()),
        "save_settings" => match argument(&request.body, "settings") {
            Some(received) => match serde_json::from_value(received) {
                Ok(settings) => result(crate::commands::settings::save_settings(settings)),
                Err(error) => Response::error(500, &format!("unreadable settings: {error}")),
            },
            None => Response::error(500, "missing the \"settings\" argument"),
        },

        // --- What the scenario decides -----------------------------------
        "status" => value(&state.account()),
        "pack_state" => value(&state.pack()),
        "news" => value(&state.feed()),
        "verify_files" => value(&Vec::<String>::new()),

        "sign_in" => sign_in(&context).await,

        "sign_out" => {
            *context.state.lock().expect("state not poisoned") = State::SignedOut;
            Response::empty()
        }

        "play" => play(&context, state, true).await,
        "install" => play(&context, state, false).await,

        // --- What makes no sense outside the window -----------------------
        //
        // `screen` renders `null`, which the Settings page knows how to
        // handle: it falls back on its fixed list of resolutions.
        // `open_folder` and `front_ready` do nothing — there's neither a
        // file explorer to open, nor a splash screen to close.
        "screen" => Response::json("null".to_string()),
        "open_folder" | "front_ready" => Response::empty(),

        // The two moments of the sign-in window. In a browser there's only
        // one tab: there's nothing to open or close, and it's the front's
        // router that carries you from one page to the other. They still
        // answer — an unknown command would return a 404 that the front
        // would open as an incident, over a gesture that simply doesn't
        // happen here.
        "open_sign_in" | "main_ready" => Response::empty(),

        // Stopping a session that doesn't exist in a browser: we answer as
        // if it were done, so the gesture stays workable here.
        "stop_game" => {
            tracing::info!("game stop requested (simulated)");
            Response::empty()
        }

        // The front logs here too, under the "browser" label: there's no
        // window to query, and the sequence stays readable in the same
        // stream as everything else.
        "log" => {
            let level = text(&request.body, "level").unwrap_or_else(|| "info".to_string());
            let message = text(&request.body, "message").unwrap_or_default();
            tracing::info!(target: "front", window = "browser", level = %level, "{message}");
            Response::empty()
        }

        // The floor of the "signed in" screen, held here too.
        //
        // In the window, it's `windows::sign_in_succeeded` that holds it —
        // at least two seconds, long enough to read your username and
        // understand that it worked. Without reproducing it, this screen
        // would flash by as a single image in a browser, and it couldn't be
        // worked on: that's exactly what this server exists to make
        // observable.
        "sign_in_succeeded" => {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            Response::empty()
        }

        _ => Response::error(404, &format!("unknown command: {name}")),
    }
}

/// Signing in, played like the real thing: the code first, the account
/// after.
///
/// The delay isn't a nicety. At Microsoft, between the moment the player
/// authorizes and the moment the launcher notices, a polling interval
/// elapses — measured at roughly four seconds. That's precisely the wait the
/// interface has to know how to inhabit, so the one it has to be possible to
/// look at.
async fn sign_in(context: &Arc<Context>) -> Response {
    context.emit(
        crate::commands::EVENT_CODE,
        json!({
            "code": "FKRD-QXZB",
            "url": "https://www.microsoft.com/link",
            "directUrl": "https://www.microsoft.com/link?otc=FKRD-QXZB",
        }),
    );

    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    *context.state.lock().expect("state not poisoned") = State::NothingInstalled;
    value(&State::NothingInstalled.account())
}

/// A session, preceded by its install when there is one.
///
/// The steps scroll by for real, with bytes climbing: it's the only way to
/// see whether the bar, the labels and the rate hold up over several
/// seconds, and not on a frozen screenshot.
async fn play(context: &Arc<Context>, state: State, played: bool) -> Response {
    use crate::phase::Phase;

    let steps = [
        (Phase::Pack, "Reading the manifest"),
        (Phase::Loader, "NeoForge 21.1.250"),
        (Phase::Minecraft, "Client, libraries and assets"),
        (Phase::Java, "Temurin 21"),
        (Phase::NeoForge, "Official installer"),
        (Phase::Mods, "128 mods"),
        (Phase::Lock, "Lock written"),
    ];

    for (index, (phase, note)) in steps.iter().enumerate() {
        let total = 840_000_000u64;
        let bytes = total * (index as u64 + 1) / steps.len() as u64;
        context.emit(
            crate::cinematic::EVENT_PROGRESS,
            json!({
                "phase": phase,
                "done": false,
                "note": note,
                "file": "sodium-neoforge-0.6.13.jar",
                "bytes": bytes,
                "total": total,
                "files": (index + 1) * 18,
                "filesTotal": 128,
                "active": true,
                "rate": 8_400_000,
                "remaining": (steps.len() - index - 1) as u64 * 9,
            }),
        );
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    }

    // The session itself — ONLY if the gesture is "play". The install
    // button stops here: that's the whole point of the split.
    if played {
        context.emit(
            crate::cinematic::EVENT_PROGRESS,
            json!({ "phase": Phase::Launch, "done": false, "active": false,
                    "bytes": 0, "total": 0, "files": 0, "filesTotal": 0, "rate": 0 }),
        );
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    *context.state.lock().expect("state not poisoned") = State::ReadyToPlay;
    context.emit(
        crate::cinematic::EVENT_PROGRESS,
        serde_json::to_value(State::ReadyToPlay.progress()).unwrap_or(serde_json::Value::Null),
    );

    value(&state.play_result(played))
}

/// What the server says about itself, for whoever opens the address by
/// hand.
fn home(state: State) -> Response {
    let current = State::ALL
        .iter()
        .find(|(_, known)| *known == state)
        .map(|(name, _)| *name)
        .unwrap_or("unknown");

    value(&scenario::Home {
        scenario: current.to_string(),
        scenarios: State::ALL
            .iter()
            .map(|(name, _)| name.to_string())
            .collect(),
        commands: vec![
            "brand",
            "path",
            "status",
            "sign_in",
            "sign_out",
            "pack_state",
            "play",
            "install",
            "verify_files",
            "news",
            "settings",
            "save_settings",
            "screen",
            "open_folder",
            "front_ready",
            "open_sign_in",
            "sign_in_succeeded",
            "main_ready",
            "log",
            "stop_game",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
    })
}

/// Unpacks a `Result`, EXACTLY like the Tauri bridge does.
///
/// **This is the one place this server could lie about the shape of what it
/// renders, and it did.** `#[tauri::command]` wraps a command that returns a
/// `Result`: success leaves as the bare value, error rejects the promise.
/// Serializing the `Result` as-is gives `{"Ok": {…}}` — a response that
/// looks like a success, that carries a 200 code, and whose front reads a
/// field that doesn't exist.
///
/// The symptom stayed unreadable for a long time: the settings page saved,
/// received an object of a shape it didn't know, and opened an incident per
/// cursor keystroke — three hundred and twenty-six in one session. Nothing
/// in the server said so, since from its point of view everything had gone
/// fine.
pub fn result<T, E>(result: Result<T, E>) -> Response
where
    T: serde::Serialize,
    E: serde::Serialize,
{
    match result {
        Ok(rendered_value) => value(&rendered_value),
        // The error is SERIALIZED, not formatted: that's what the bridge
        // does, rejecting with the error value as-is. `Error` is a newtype
        // over a string, so this renders a JSON string — the exact shape
        // `errorMessage` knows how to read on both sides.
        Err(error) => match serde_json::to_string(&error) {
            Ok(body) => Response {
                code: 500,
                mime_type: "application/json".to_string(),
                body,
            },
            Err(cause) => Response::error(500, &format!("unserializable error: {cause}")),
        },
    }
}

/// Serializes, or renders the failure rather than hiding it.
fn value<T: serde::Serialize>(value: &T) -> Response {
    match serde_json::to_string(value) {
        Ok(body) => Response::json(body),
        Err(error) => Response::error(500, &format!("unserializable response: {error}")),
    }
}

/// A named argument, in the JSON body.
///
/// The front sends what `invoke` would send: an object whose keys are the
/// parameter names. We read them the same way.
pub fn argument(body: &str, name: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(name)
        .cloned()
}

/// The same, when the expected argument is a string.
///
/// A missing argument or one of another type returns `None` rather than
/// failing: this server exists to work on the interface, and a malformed log
/// line shouldn't interrupt the gesture being observed.
pub fn text(body: &str, name: &str) -> Option<String> {
    argument(body, name)?.as_str().map(str::to_owned)
}

#[cfg(test)]
#[path = "dev.test.rs"]
mod tests;
