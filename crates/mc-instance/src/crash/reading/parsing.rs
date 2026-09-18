//! Recognize a Java exception in a log line.

/// Recognizes `package.Class: message` in a line.
///
/// A Java trace names its class with a dotted path ending in a capitalized
/// identifier. Requiring both avoids mistaking a timestamp or a file path
/// for an exception, since those also contain dots and colons.
pub(crate) fn split_exception(line: &str) -> Option<(String, String)> {
    let line = strip_ansi(line);
    let line = line.trim();
    // Trace lines start with "at": those are frames, not the exception
    // declaration.
    if line.starts_with("at ") {
        return None;
    }

    let candidate = match line.find("Caused by: ") {
        Some(pos) => &line[pos + "Caused by: ".len()..],
        None => line,
    };
    // A log line prefixed with a timestamp and a category: only what
    // follows the last "]: " is kept.
    let candidate = match candidate.rfind("]: ") {
        Some(pos) => &candidate[pos + 3..],
        None => candidate,
    };

    let (class, message) = candidate.split_once(':')?;
    let class = class.trim();
    if !class.contains('.') || class.contains(' ') || class.contains('/') {
        return None;
    }
    let last = class.rsplit('.').next()?;
    if !last.chars().next()?.is_ascii_uppercase() {
        return None;
    }
    // Extra safety net: an exception class name almost always ends this
    // way, and stopping there rules out the remaining false positives.
    if !(last.ends_with("Exception") || last.ends_with("Error") || last.ends_with("Throwable")) {
        return None;
    }

    Some((class.to_string(), message.trim().to_string()))
}

/// Strips ANSI color sequences.
///
/// Minecraft colors its output; left in, these sequences make the excerpt
/// unreadable in an incident report.
pub(crate) fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "parsing.test.rs"]
mod tests;
