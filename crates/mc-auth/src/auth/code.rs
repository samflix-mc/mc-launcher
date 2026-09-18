//! The code the user enters to authorize the launcher.

/// What's shown to the user so they authorize the launcher.
///
/// `direct_verification_uri` prefills the code into the URL: a click instead
/// of a copy-paste. Both are given, because a terminal that doesn't open a
/// browser needs the long form.
#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub direct_verification_uri: String,
}
