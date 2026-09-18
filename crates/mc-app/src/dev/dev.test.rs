use super::http::Request;
use super::scenario::State;
use super::{Context, argument, router};

fn request(path: &str) -> Request {
    Request {
        method: "POST".to_string(),
        path: path.to_string(),
        body: String::new(),
    }
}

/// **The server's contract: the same names as the bridge's.**
///
/// If a name drifts, the front gets a 404 for a command that exists — and
/// the symptom is a blank screen with no message, since `invoke` and `fetch`
/// fail silently the same way.
#[tokio::test]
async fn all_the_bridge_commands_answer() {
    let context = Context::new(State::ReadyToPlay);

    for name in [
        "brand",
        "path",
        "status",
        "pack_state",
        "news",
        "settings",
        "verify_files",
        "screen",
        "open_folder",
        "front_ready",
        "open_sign_in",
        "sign_in_succeeded",
        "main_ready",
    ] {
        let response = router(context.clone(), request(&format!("/command/{name}"))).await;
        assert_eq!(response.code, 200, "{name}: {}", response.body);
    }
}

#[tokio::test]
async fn an_unknown_command_says_so() {
    let context = Context::new(State::ReadyToPlay);
    let response = router(context, request("/command/crash")).await;

    assert_eq!(response.code, 404);
    assert!(response.body.contains("crash"), "{}", response.body);
}

/// The scenario changes in one request, and the next response takes it into
/// account. That's the whole point of the server: reaching a state without
/// having to trigger it for real.
#[tokio::test]
async fn the_scenario_changes_and_what_follows_tracks_it() {
    let context = Context::new(State::SignedOut);

    let before = router(context.clone(), request("/command/status")).await;
    assert_eq!(before.body, "null", "nobody is signed in at the start");

    let switch = router(context.clone(), request("/scenario/ready-to-play")).await;
    assert_eq!(switch.code, 200);

    let after = router(context.clone(), request("/command/status")).await;
    assert!(after.body.contains("thesam1798"), "{}", after.body);
}

#[tokio::test]
async fn an_unknown_scenario_says_so() {
    let context = Context::new(State::SignedOut);
    let response = router(context, request("/scenario/whatever")).await;

    assert_eq!(response.code, 404);
}

/// Every scenario must be reachable by its name: a name in the list that
/// `from_name` doesn't recognize would be an announced but unreachable
/// scenario.
#[tokio::test]
async fn every_announced_scenario_is_reachable() {
    let context = Context::new(State::SignedOut);

    for (name, expected) in State::ALL {
        let response = router(context.clone(), request(&format!("/scenario/{name}"))).await;
        assert_eq!(response.code, 200, "{name}");
        assert_eq!(
            *context.state.lock().expect("state"),
            expected,
            "{name} did not set the state it announces"
        );
    }
}

/// The account with NO LICENSE is a state in its own right, not the absence
/// of an account: it's the one the Sign In page has to know how to display.
#[tokio::test]
async fn no_license_renders_an_account_that_does_not_own_the_game() {
    let context = Context::new(State::NoLicense);
    let response = router(context, request("/command/status")).await;

    assert!(
        response.body.contains("\"ownsTheGame\":false"),
        "{}",
        response.body
    );
}

/// The root announces what can be requested. It's the documentation you
/// read when you've forgotten the names — so it has to be exact.
#[tokio::test]
async fn the_root_announces_the_scenarios_and_the_commands() {
    let context = Context::new(State::Offline);
    let response = router(context, request("/")).await;

    assert_eq!(response.code, 200);
    assert!(response.body.contains("offline"), "{}", response.body);
    assert!(response.body.contains("pack_state"));
}

/// Arguments arrive as `invoke` would send them: an object whose keys are
/// the parameter names.
#[test]
fn a_named_argument_reads_from_the_body() {
    assert_eq!(
        argument(r#"{"deep":true}"#, "deep"),
        Some(serde_json::Value::Bool(true))
    );
    assert_eq!(argument(r#"{"other":1}"#, "deep"), None);
    assert_eq!(argument("not json", "deep"), None);
}

/// The feed is empty ONLY offline: everywhere else, the news page has to
/// have content to show — it's the only way to look at it as long as
/// mc-content hasn't published anything.
#[tokio::test]
async fn the_feed_carries_posts_except_offline() {
    let with_news = Context::new(State::ReadyToPlay);
    let response = router(with_news, request("/command/news")).await;
    assert!(
        response.body.contains("The launcher is here"),
        "{}",
        response.body
    );

    let without_news = Context::new(State::Offline);
    let response = router(without_news, request("/command/news")).await;
    assert!(response.body.contains("\"posts\":[]"), "{}", response.body);
}

/// A post's body is a TREE, never HTML: that's the condition under which the
/// CSP was loosened, and it holds for the demo data too.
#[tokio::test]
async fn the_demo_posts_are_trees() {
    let context = Context::new(State::ReadyToPlay);
    let response = router(context, request("/command/news")).await;

    assert!(
        response.body.contains("\"type\":\"paragraph\""),
        "{}",
        response.body
    );
    assert!(!response.body.contains("<p>"), "HTML leaked into the body");
}

/// **The shape of what a command that can fail renders.**
///
/// `#[tauri::command]` unwraps the `Result`: success leaves as the BARE
/// value, error rejects with the error value as-is. Serializing the
/// `Result` as-is used to give `{"Ok": {…}}` — a response that looks like a
/// success, that carries a 200 code, and whose front reads a field that
/// doesn't exist.
///
/// The symptom was unreadable: the settings page opened an incident per
/// cursor keystroke — three hundred and twenty-six in one session — and
/// nothing in the server said so, since from its point of view everything
/// had gone fine.
///
/// The test targets the function and not a command, and that's deliberate:
/// `save_settings` REALLY writes to the developer's disk, and a test that
/// called it would replace their preferences with the defaults.
#[test]
fn a_result_is_unwrapped_like_the_bridge_does_it() {
    let good: Result<Vec<&str>, String> = Ok(vec!["a", "b"]);
    let rendered = super::result(good);
    assert_eq!(rendered.code, 200);
    assert_eq!(rendered.body, r#"["a","b"]"#, "the value must leave BARE");

    let bad: Result<Vec<&str>, String> = Err("the disk is full".to_string());
    let rendered = super::result(bad);
    assert_eq!(rendered.code, 500);
    assert_eq!(
        rendered.body, "\"the disk is full\"",
        "the error must have the SAME shape as the bridge's: a JSON string"
    );
}
