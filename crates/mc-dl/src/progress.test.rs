//! What the observer actually receives, against a real server.
//!
//! Nothing here checks formatting: it's the emission contract that's at
//! stake. If bytes stop being announced as they come in, the window is back
//! to several minutes of silence with no compiler complaint about it.

use std::sync::{Arc, Mutex};

use crate::{Check, Checksum, Downloader, Fetched};

/// A copied `Progress`, because the original borrows the file name for the
/// duration of the call and can't be kept.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Trace {
    Batch {
        files: usize,
        bytes: u64,
    },
    Started {
        file: String,
        bytes: Option<u64>,
    },
    Received(u64),
    Lost(u64),
    Finished {
        file: String,
        state: Fetched,
        bytes: u64,
    },
}

#[derive(Default)]
struct Log(Mutex<Vec<Trace>>);

impl Log {
    fn observer(self: &Arc<Self>) -> crate::Observer {
        let log = Arc::clone(self);
        Arc::new(move |progress| {
            let trace = match progress {
                super::Progress::Batch { files, bytes } => Trace::Batch { files, bytes },
                super::Progress::Started { file, bytes } => Trace::Started {
                    file: file.to_string(),
                    bytes,
                },
                super::Progress::Received(n) => Trace::Received(n),
                super::Progress::Lost(n) => Trace::Lost(n),
                super::Progress::Finished { file, state, bytes } => Trace::Finished {
                    file: file.to_string(),
                    state,
                    bytes,
                },
            };
            log.0.lock().expect("log").push(trace);
        })
    }

    fn traces(&self) -> Vec<Trace> {
        self.0.lock().expect("log").clone()
    }

    /// What actually transited over the network, retries subtracted.
    fn net_bytes(&self) -> i64 {
        self.traces()
            .iter()
            .map(|trace| match trace {
                Trace::Received(n) => *n as i64,
                Trace::Lost(n) => -(*n as i64),
                _ => 0,
            })
            .sum()
    }
}

fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-dl-progress-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&path).ok();
    path
}

fn sha1(bytes: &[u8]) -> Checksum {
    Checksum::Sha1(Checksum::Sha1(String::new()).of(bytes))
}

#[tokio::test]
async fn bytes_are_announced_during_the_download() {
    let server = mc_testkit::Server::new().await;
    let body = vec![b'x'; 64 * 1024];
    server.bytes("/big.jar", &body);

    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());

    dl.bytes(&server.url("/big.jar")).await.unwrap();

    // The total matters more than the split: the number of chunks depends
    // on the TCP stack and doesn't need to be pinned down in a test.
    assert_eq!(log.net_bytes(), body.len() as i64);
    assert!(
        log.traces().iter().any(|t| matches!(t, Trace::Received(_))),
        "no bytes announced: {:?}",
        log.traces()
    );
}

/// A 500 comes back before the body even starts: nothing should have been
/// counted, or the retry would double the bar.
#[tokio::test]
async fn a_refused_response_counts_nothing() {
    let server = mc_testkit::Server::new().await;
    server.code("/dead", 500);

    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());

    dl.bytes(&server.url("/dead")).await.expect_err("500");

    assert_eq!(log.net_bytes(), 0, "{:?}", log.traces());
}

/// Three attempts, two of them failed: only the body finally obtained
/// should have been counted.
#[tokio::test]
async fn a_retry_does_not_count_twice() {
    let server = mc_testkit::Server::new().await;
    server.fails_then("/flaky", 2, r#"{"ok":true}"#);

    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());

    let received = dl.bytes(&server.url("/flaky")).await.unwrap();

    assert_eq!(log.net_bytes(), received.len() as i64);
}

#[tokio::test]
async fn a_downloaded_file_is_bracketed_by_its_name() {
    let server = mc_testkit::Server::new().await;
    let body = b"mod content";
    server.bytes("/jei.jar", body);

    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());
    let dest = folder("download").join("mods/jei-1.21.1.jar");

    dl.to_file(&server.url("/jei.jar"), &dest, Check::Full(&sha1(body)))
        .await
        .unwrap();

    let traces = log.traces();
    // The short name, not the path: that's what fits on one line.
    assert_eq!(
        traces.first(),
        Some(&Trace::Started {
            file: "jei-1.21.1.jar".to_string(),
            bytes: None,
        }),
        "{traces:?}"
    );
    assert_eq!(
        traces.last(),
        Some(&Trace::Finished {
            file: "jei-1.21.1.jar".to_string(),
            state: Fetched::Downloaded,
            bytes: body.len() as u64,
        }),
        "{traces:?}"
    );
}

/// The reinstall case: everything is already there, nothing comes down, and
/// the bar must still reach the end.
#[tokio::test]
async fn an_already_present_file_counts_its_weight_without_downloading_anything() {
    let server = mc_testkit::Server::new().await;
    let body = b"already there";
    server.bytes("/present.jar", body);

    let dest = folder("present").join("present.jar");
    crate::write_atomic(&dest, body).unwrap();

    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());

    let state = dl
        .to_file(&server.url("/present.jar"), &dest, Check::Full(&sha1(body)))
        .await
        .unwrap();

    assert_eq!(state, Fetched::AlreadyPresent);
    assert_eq!(log.net_bytes(), 0, "nothing should have transited");
    assert_eq!(
        log.traces().last(),
        Some(&Trace::Finished {
            file: "present.jar".to_string(),
            state: Fetched::AlreadyPresent,
            // The size comes from disk: `Check::Full` doesn't publish one,
            // and zero here would leave the bar stuck.
            bytes: body.len() as u64,
        }),
        "{:?}",
        log.traces()
    );
}

/// Batches aren't counted in `mc-dl`: it's the caller who knows how many
/// files it's about to request, and without this announcement no time
/// remaining is computable.
#[tokio::test]
async fn a_batch_announced_by_the_caller_passes_through_unchanged() {
    let log = Arc::new(Log::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(log.observer());

    dl.emit(super::Progress::Batch {
        files: 2_500,
        bytes: 830_000_000,
    });

    assert_eq!(
        log.traces(),
        vec![Trace::Batch {
            files: 2_500,
            bytes: 830_000_000
        }]
    );
}

/// Without an observer — the command-line case — nothing should behave
/// differently.
#[tokio::test]
async fn without_an_observer_the_download_behaves_the_same_way() {
    let server = mc_testkit::Server::new().await;
    server.json("/list", r#"{"ok":true}"#);

    let bytes = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .bytes(&server.url("/list"))
        .await
        .unwrap();

    assert_eq!(bytes, br#"{"ok":true}"#);
}
