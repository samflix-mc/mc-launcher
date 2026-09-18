use super::{RedactingWriter, file_layer_in};

/// A working directory specific to this test, removed by the caller.
fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-log-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&path).ok();
    path
}

#[test]
fn the_log_file_does_not_receive_secrets() {
    use std::io::Write;

    // The dreaded scenario: a player attaches their log to a ticket. What's
    // written to disk must already be redacted, not just what goes to
    // Sentry.
    let mut buffer = Vec::new();
    {
        let mut writer = RedactingWriter { inner: &mut buffer };
        writeln!(
            writer,
            "INFO exchange succeeded access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ"
        )
        .unwrap();
    }

    let written = String::from_utf8(buffer).unwrap();
    assert!(
        !written.contains("eyJhbGci"),
        "token written in the clear: {written}"
    );
    assert!(written.contains("[secret]"));
    assert!(written.contains("exchange succeeded"));
}

#[test]
fn the_writer_returns_the_number_of_bytes_it_was_given() {
    use std::io::Write;

    // Redaction shortens the text. Returning the written length would look
    // like a partial write, and `write_all` would loop on it forever.
    let mut buffer = Vec::new();
    let mut writer = RedactingWriter { inner: &mut buffer };
    let line = b"access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ";
    assert_eq!(writer.write(line).unwrap(), line.len());
    writer.flush().unwrap();
}

#[test]
fn the_file_layer_opens_the_days_log() {
    let dir = folder("layer");
    let (layer, guard, returned) = file_layer_in(dir.clone(), "mc-fixture");

    assert!(layer.is_some(), "no layer");
    assert_eq!(returned.as_deref(), Some(dir.as_path()));
    // The directory is returned, not the file: it's the appender that
    // decides the day's name, and it changes at midnight.
    assert!(dir.is_dir());

    drop(guard);
    drop(layer);
    std::fs::remove_dir_all(&dir).ok();
}

/// A failure to open it must not stop the program from running: we lose the
/// log, not the install.
#[test]
fn an_impossible_directory_does_not_stop_the_program() {
    let blocking = folder("blocking");
    // An ordinary file where a directory is expected: `create_dir_all`
    // fails, exactly as it would on a full or read-only disk.
    std::fs::write(&blocking, b"not a directory").unwrap();

    let (layer, guard, returned) = file_layer_in(blocking.join("logs"), "mc-fixture");
    assert!(layer.is_none());
    assert!(guard.is_none());
    assert!(returned.is_none());

    std::fs::remove_file(&blocking).ok();
}

/// `file_layer` does exactly one thing more than `file_layer_in`: choose the
/// directory. That's little, and it's everything that puts the log where
/// diagnosis will look for it — returning three `None`s would silently
/// deprive the launcher of its file, the very one a player is asked to
/// attach.
#[test]
fn the_file_layer_installs_itself_in_the_logs_directory() {
    let vars = crate::fixtures::variables();
    let root = folder("layer-default");
    std::fs::create_dir_all(&root).unwrap();
    vars.set("XDG_DATA_HOME", root.to_str().unwrap());

    let (layer, guard, path) = super::file_layer("fixture");

    assert!(layer.is_some(), "no file layer");
    assert!(guard.is_some(), "no write guard");
    let path = path.expect("the logs directory is returned");
    assert!(
        path.starts_with(&root),
        "log outside the declared directory: {path:?}"
    );
    assert_eq!(path, crate::guard::log_dir());

    drop(guard);
    std::fs::remove_dir_all(&root).ok();
}

/// A `flush` that doesn't reach down to the file leaves the last line stuck
/// in a buffer — and that's precisely the one that says why the launcher
/// stopped.
#[test]
fn the_flush_travels_through_to_the_wrapped_writer() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Witness(Arc<Mutex<usize>>);

    impl Write for Witness {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }

    let witness = Witness::default();
    let mut writer = RedactingWriter {
        inner: witness.clone(),
    };
    writer.write_all(b"a line\n").unwrap();
    assert_eq!(*witness.0.lock().unwrap(), 0, "nothing was asked yet");

    writer.flush().unwrap();
    assert_eq!(*witness.0.lock().unwrap(), 1, "the flush didn't go through");
}
