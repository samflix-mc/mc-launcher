use std::time::Duration;

use tokio::net::TcpListener;

use super::ping;

/// What [`connection::exchange`](crate::connection) doesn't cover: the
/// deadline itself. A server that accepts the connection and then says
/// nothing must not hold this call open past `timeout`.
#[tokio::test]
async fn a_server_that_never_answers_times_out() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        // Held open well past the timeout below, on purpose: this must be
        // what stops the call, not the socket closing on its own.
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let started = std::time::Instant::now();
    let result = ping("127.0.0.1", port, Duration::from_millis(100)).await;

    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the timeout must bound the wait, not the far-off sleep"
    );
}

#[tokio::test]
async fn nothing_listening_fails_rather_than_waiting_out_the_timeout() {
    // Port 0 on connect is refused synchronously by the OS on every
    // platform this launcher targets: nothing ever gets to listen there.
    let result = ping("127.0.0.1", 0, Duration::from_secs(2)).await;
    assert!(result.is_err());
}
