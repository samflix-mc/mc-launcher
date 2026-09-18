//! The command line produced, and how it's displayed without revealing everything.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Command {
    pub java: PathBuf,
    pub args: Vec<String>,
    /// Working directory: the game writes `saves`, `logs`, `options.txt` there.
    pub working_dir: PathBuf,
}

impl Command {
    /// Readable command line, for display or for replaying by hand.
    ///
    /// The classpath is abbreviated: it runs to several tens of thousands
    /// of characters and no one reads it.
    pub fn display(&self) -> String {
        let mut out = vec![self.java.display().to_string()];
        let mut skip_next = false;
        for arg in &self.args {
            if skip_next {
                out.push(format!("<{} libraries>", arg.split(':').count()));
                skip_next = false;
                continue;
            }
            if arg == "-cp" {
                skip_next = true;
            }
            out.push(arg.clone());
        }
        out.join(" ")
    }
}

// --- Reading descriptors ------------------------------------------------------

#[cfg(test)]
#[path = "command.test.rs"]
mod tests;
