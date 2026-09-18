//! What a runtime is, and how its version is measured.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A Java runtime found or installed, whose version has been **measured** by
/// running the binary — never inferred from its path.
#[derive(Debug, Clone)]
pub struct Java {
    /// `java` executable (`java.exe` on Windows).
    pub path: PathBuf,
    pub version: Version,
    pub origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Installed by the launcher in its own directory.
    Managed,
    /// Found on the system (`JAVA_HOME`, `PATH`, usual locations).
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    /// Full string as reported, e.g. `21.0.5+11`.
    pub full: String,
}

/// Extracts the major number from a Java version string.
///
/// Two schemes still coexist: `1.8.0_412` (up to Java 8, where the major is
/// the *second* number) and `21.0.5` (since Java 9). Mixing up the two would
/// pass a Java 8 off as a Java 1.
pub fn parse_major(version: &str) -> Option<u32> {
    let cleaned: String = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .to_string();
    let mut parts = cleaned.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Queries a `java` executable and reads its version.
///
/// `-version` writes to **stderr** (a historical JVM choice) and across three
/// lines, only the first of which carries the number, in quotes.
pub async fn probe(exe: &Path) -> Result<Version> {
    let out = tokio::process::Command::new(exe)
        .arg("-version")
        .output()
        .await
        .with_context(|| format!("running {}", exe.display()))?;

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let quoted = text
        .split('"')
        .nth(1)
        .with_context(|| format!("unreadable version in the output of {}", exe.display()))?;
    let major =
        parse_major(quoted).with_context(|| format!("unreadable major number in \"{quoted}\""))?;

    Ok(Version {
        major,
        full: quoted.to_string(),
    })
}

#[cfg(test)]
#[path = "version.test.rs"]
mod tests;
