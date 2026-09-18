use super::{Auth, client};

/// The client has one role: identify the launcher and fix the TLS stack.
/// `minecraft-auth` sets its own timeouts per request.
///
/// Identification isn't a courtesy. It's the name Microsoft and Mojang
/// recognize whoever's knocking at their door by, and an anonymous client
/// gets throttled before it gets refused — an outage that only happens in
/// production, and only when there's a crowd.
#[tokio::test]
async fn the_authentication_client_names_itself() {
    let server = mc_testkit::Server::new().await;
    server.json("/who-are-you", "{}");

    let client = client().expect("the client builds");
    client
        .get(server.url("/who-are-you"))
        .send()
        .await
        .expect("the test server responds");

    let received = server.received();
    let request = received.first().expect("a request arrived");
    let name = request
        .header("user-agent")
        .expect("no user-agent: the launcher would knock anonymously");
    assert!(
        name.starts_with("samflix-mc-launcher/"),
        "unexpected identity: {name}"
    );
}

/// An unreadable session must not panic the launch: it means "sign in
/// again", and the message must say so.
#[test]
fn an_unreadable_session_says_so_instead_of_panicking() {
    let error = failure(&serde_json::json!({"whatever": "this is"}));

    assert!(error.contains("saved session is unreadable"), "{error}");
}

#[test]
fn a_state_that_is_not_an_object_is_also_refused() {
    failure(&serde_json::Value::Null);
    failure(&serde_json::json!([1, 2, 3]));
}

/// `Auth` isn't `Debug` — it holds tokens, and that's deliberate: `{:?}` on
/// this struct would write them to a log. Hence this manual match instead of
/// an `expect_err`.
fn failure(state: &serde_json::Value) -> String {
    match Auth::resume(state) {
        Ok(_) => panic!("this isn't a JavaAuthManager state"),
        Err(error) => format!("{error:#}"),
    }
}
