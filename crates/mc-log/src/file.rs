//! The file layer: full detail, redacted, with rotation.

use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, Layer};

use crate::BoxedLayer;
use crate::guard::{log_dir, purge_old_logs};
use crate::redact::redact;

/// Writer that redacts each line before writing it.
///
/// The log file is exactly what we ask a player to attach when something
/// fails — on a Discord channel, in a ticket. If it contains their Microsoft
/// token, we've created the problem we were trying to avoid. Redaction
/// therefore applies to the file as much as to Sentry, not just to what
/// goes out over the network.
pub(crate) struct RedactingWriter<W> {
    inner: W,
}

impl<W: std::io::Write> std::io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // `tracing` hands back one whole formatted event per call: redaction
        // therefore always sees complete lines, never a token cut in half.
        let text = String::from_utf8_lossy(buf);
        self.inner.write_all(redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) struct Redacting<M>(pub(crate) M);

impl<'a, M> tracing_subscriber::fmt::MakeWriter<'a> for Redacting<M>
where
    M: tracing_subscriber::fmt::MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.0.make_writer(),
        }
    }
}

/// File layer, with daily rotation.
///
/// A failure to open it must not stop the program from running: the
/// directory could be read-only or full. We lose the log, not the install.
///
/// Returns the directory, not the file: it's the appender that decides the
/// day's name, and it changes at midnight.
pub(crate) fn file_layer(
    component: &str,
) -> (
    Option<BoxedLayer>,
    Option<tracing_appender::non_blocking::WorkerGuard>,
    Option<PathBuf>,
) {
    file_layer_in(log_dir(), component)
}

/// Same thing, in a given directory.
///
/// The directory is an argument rather than a read of [`log_dir`]: it's the
/// only way to check that a directory that can't be created does yield
/// three `None`s instead of stopping the program — and to check it
/// elsewhere than in the logs of the machine running the tests.
pub(crate) fn file_layer_in(
    dir: PathBuf,
    component: &str,
) -> (
    Option<BoxedLayer>,
    Option<tracing_appender::non_blocking::WorkerGuard>,
    Option<PathBuf>,
) {
    if std::fs::create_dir_all(&dir).is_err() {
        return (None, None, None);
    }
    purge_old_logs(&dir);

    let appender = tracing_appender::rolling::daily(&dir, format!("{component}.log"));
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(Redacting(writer))
        // A file reread later, often by someone else: no colors, and the
        // message's target is needed to place it.
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("debug,hyper=info,reqwest=info,rustls=info"))
        .boxed();

    (Some(layer), Some(guard), Some(dir))
}

#[cfg(test)]
#[path = "file.test.rs"]
mod tests;
