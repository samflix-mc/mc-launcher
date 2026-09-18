//! Scrubbing secrets before a text leaves the machine.
//!
//! This launcher handles Microsoft access tokens, Xbox Live tokens, a
//! Minecraft token and a CurseForge API key. An incident report is free
//! text: error message, file path, URL, call stack. Nothing stops a
//! token from ending up in it — a signed URL, a struct's `Debug`, an API
//! message echoing the request.
//!
//! The filter applies to **everything** headed for Sentry, both events
//! and breadcrumbs, and to the log file. It doesn't try to be clever: it
//! spots a small number of known shapes and replaces them outright.
//! Making an incident a bit less readable is nothing next to leaking a
//! token that gives access to a Microsoft account.
mod cursor;
mod home;
mod jwt;
mod keyword;
mod tables;

/// What replaces a secret.
pub(crate) const MASK: &str = "[secret]";

/// Replaces the secrets in a text.
///
/// Three passes that don't overlap: shape first — a JWT is recognized on
/// its own —, then what a keyword announces, then the home path.
pub fn redact(text: &str) -> String {
    let text = jwt::redact_jwt(text);
    let text = keyword::redact_after_keywords(&text);
    home::redact_home(&text)
}

#[cfg(test)]
#[path = "redact.test.rs"]
mod tests;
