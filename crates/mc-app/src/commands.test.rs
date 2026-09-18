//! What the commands promise to the window.
//!
//! The commands that matter talk to Microsoft, Mojang or the keyring: what
//! can be verified without an account is the serialization contract and the
//! few decisions made locally. A field's name is what the TypeScript reads,
//! and renaming it breaks the display without breaking compilation on
//! either side.

use super::{
    Account, DEVICE_CODE_EVENT, DeviceCode, DeviceCodeView, Error, Phase, Session, path, verdict,
};
use mc_auth::Profile;

fn session(name: &str, id: &str) -> Session {
    Session {
        minecraft_token: "token".to_string(),
        profile: Profile {
            id: id.to_string(),
            name: name.to_string(),
        },
    }
}

#[test]
fn the_account_takes_the_session_s_profile() {
    let account = Account::from((&session("Sam", "0123456789abcdef"), true));

    assert_eq!(account.username, "Sam");
    assert_eq!(account.uuid, "0123456789abcdef");
    assert!(account.owns_the_game);
}

#[test]
fn missing_license_is_passed_through_as_is() {
    // The account is valid, the sign-in succeeded: a `false` must still
    // reach the window, not an error nor a default `true`. That's what
    // saves installing eight hundred megabytes for nothing.
    let account = Account::from((&session("Sam", "abc"), false));

    assert!(!account.owns_the_game);
}

#[test]
fn the_account_serializes_in_camel_case() {
    let account = Account::from((&session("Sam", "abc"), true));

    let json = serde_json::to_value(&account).expect("serialization");
    assert_eq!(json["username"], "Sam");
    assert_eq!(json["uuid"], "abc");
    assert_eq!(json["ownsTheGame"], true);
}

#[test]
fn the_device_code_keeps_its_two_addresses() {
    // The long form is used when the browser fails to open; forwarding only
    // the direct one would leave the player with no recourse.
    let code = DeviceCodeView::from(&DeviceCode {
        user_code: "ABCD-EFGH".to_string(),
        verification_uri: "https://microsoft.com/link".to_string(),
        direct_verification_uri: "https://microsoft.com/link?otc=ABCD-EFGH".to_string(),
    });

    assert_eq!(code.code, "ABCD-EFGH");
    assert_eq!(code.url, "https://microsoft.com/link");
    assert_eq!(code.direct_url, "https://microsoft.com/link?otc=ABCD-EFGH");

    let json = serde_json::to_value(&code).expect("serialization");
    assert_eq!(
        json["directUrl"],
        "https://microsoft.com/link?otc=ABCD-EFGH"
    );
}

#[test]
fn the_error_keeps_its_whole_chain() {
    let error = Error::from(anyhow::anyhow!("the code has expired").context("Microsoft sign-in"));

    assert_eq!(
        serde_json::to_value(&error).expect("serialization"),
        serde_json::json!("Microsoft sign-in : the code has expired")
    );
}

#[test]
fn an_error_without_context_stays_its_message() {
    let error = Error::from(anyhow::anyhow!("keyring locked"));

    assert_eq!(error, Error("keyring locked".to_string()));
}

#[test]
fn the_path_is_complete_and_ordered() {
    // The window asks for it once, at opening, to draw what's left to do. A
    // partial path would never show the end.
    let path = path();

    assert_eq!(path.len(), Phase::ALL.len());
    let ranks: Vec<usize> = path.iter().map(|step| step.rank).collect();
    assert_eq!(ranks, (0..Phase::ALL.len()).collect::<Vec<_>>());
    assert!(path.iter().all(|step| !step.label.is_empty()));
}

#[test]
fn the_path_serializes_with_its_labels() {
    let json = serde_json::to_value(path()).expect("serialization");

    assert_eq!(json[0]["phase"], "signin");
    assert_eq!(json[0]["label"], "Microsoft account");
}

#[test]
fn closing_the_game_is_not_a_crash() {
    // A player who quits their session shouldn't see a red banner. Only a
    // non-zero exit code is one.
    let report = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Normal,
        errors: Vec::new(),
    };
    assert_eq!(verdict(&report), "Session ended.");

    let interrupted = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Interrupted { signal: 15 },
        errors: Vec::new(),
    };
    assert_eq!(verdict(&interrupted), "Game closed.");
}

#[test]
fn a_crash_is_named_with_its_code() {
    let report = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Failed { code: 1 },
        errors: Vec::new(),
    };

    let said = verdict(&report);
    assert!(said.contains("error"), "{said}");
    assert!(said.contains('1'), "{said}");
}

#[test]
fn the_event_name_does_not_move() {
    // The TypeScript listens for that exact string. Renaming it on only one
    // side leaves a window waiting forever for a code that was already
    // emitted.
    assert_eq!(DEVICE_CODE_EVENT, "auth://code");
}
