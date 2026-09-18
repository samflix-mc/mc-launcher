//! The launcher without its window, served over HTTP.
//!
//! ```sh
//! cargo run -p mc-app --features dev-server --bin mc-dev-server
//! pnpm --dir web start          # in another terminal
//! ```
//!
//! The front then opens in an ordinary browser, with its hot reload, and
//! talks to this server instead of `invoke`.
//!
//! ```sh
//! curl localhost:1421                              # the known scenarios
//! curl -X POST localhost:1421/scenario/installing   # change state
//! ```
//!
//! This binary only exists behind the `dev-server` feature, which isn't
//! enabled by default: `cargo tauri build` doesn't compile it.

use std::sync::Arc;

use mc_app_lib::dev;

#[tokio::main]
async fn main() {
    let _log = mc_log::init("mc-dev-server");

    let start = std::env::args()
        .nth(1)
        .and_then(|name| dev::scenario::State::from_name(&name))
        .unwrap_or(dev::scenario::State::SignedOut);

    let context = dev::Context::new(start);
    let events = context.events.clone();

    let router: dev::http::Router = Arc::new(move |request| {
        let context = Arc::clone(&context);
        Box::pin(async move { dev::router(context, request).await })
    });

    let port = std::env::var("MC_DEV_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(dev::PORT);

    println!("Development server: http://127.0.0.1:{port}");
    println!("Starting scenario: {start:?}");
    println!("Known scenarios: curl localhost:{port}");

    if let Err(error) = dev::http::serve(port, router, events).await {
        eprintln!("the server stopped: {error}");
        std::process::exit(1);
    }
}
