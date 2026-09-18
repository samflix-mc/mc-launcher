//! How the game ended, and what we say about it.

/// What one run of the game produced.
#[derive(Debug)]
pub struct Report {
    pub outcome: Outcome,
    /// Exceptions picked out of the output, whether or not the game
    /// crashed.
    ///
    /// Minecraft catches a lot of them and carries on: those errors don't
    /// show up anywhere else, and they're often the ones that explain a
    /// behavior reported much later.
    pub errors: Vec<crate::crash::Crash>,
}

/// How the game ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Normal exit, end screen or window closed.
    Normal,
    /// Stop requested from the outside: `Ctrl+C`, `kill`, session closing.
    ///
    /// Not a crash. Confusing it with an error used to fill the incident
    /// dashboard every time the game was closed.
    Interrupted { signal: i32 },
    /// The game stopped on its own on an error.
    Failed { code: i32 },
}

impl Outcome {
    pub(super) fn from_status(status: &std::process::ExitStatus) -> Outcome {
        if status.success() {
            return Outcome::Normal;
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Outcome::Interrupted { signal };
            }
            // A shell translates a signal into 128 + n. `status.signal()`
            // doesn't see it when the code crosses an intermediary, hence
            // this second reading: 143 is a SIGTERM, 130 a Ctrl+C.
            if let Some(code) = status.code()
                && (129..=192).contains(&code)
            {
                return Outcome::Interrupted { signal: code - 128 };
            }
        }

        Outcome::failure(status.code())
    }

    /// The failure, with the code the system returned — or, failing that, a
    /// code that no program can produce.
    ///
    /// The fallback is unreachable on Unix: a missing code means a signal,
    /// handled above. It exists for platforms that don't have one, and is
    /// checked here rather than through a status we don't know how to
    /// build. Returning 1 instead would pass an unexplained stop off as an
    /// ordinary game error.
    fn failure(code: Option<i32>) -> Outcome {
        Outcome::Failed {
            code: code.unwrap_or(-1),
        }
    }

    /// Is there cause to open an incident?
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failed { .. })
    }
}

#[cfg(test)]
#[path = "report.test.rs"]
mod tests;
