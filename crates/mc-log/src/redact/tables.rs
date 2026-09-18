//! What announces a secret, and what only announces its announcement.

/// Words that announce a sensitive value right after them.
pub(super) const KEYWORDS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "device_code",
    "minecraft_token",
    "api_key",
    "apikey",
    "api-key",
    "x-api-key",
    "x-api-token",
    "authorization",
    "password",
    "secret",
    "token",
];

/// HTTP authentication schemes: the word announces the value, it isn't
/// the value.
///
/// Without this list, only "Bearer" was recognized. Other schemes were
/// mistaken for the secret itself: the scheme name got masked, and the
/// identifier following it went out in the clear.
///
/// A scheme is only recognized if it forms a word on its own *and*
/// actually introduces something; otherwise it is the value. Without
/// both conditions, "token=basicXXXX" let "basic" through in the clear,
/// and "password: digest" masked nothing at all.
pub(super) const SCHEMES: &[&str] = &["bearer", "basic", "digest", "negotiate", "token", "dpop"];
