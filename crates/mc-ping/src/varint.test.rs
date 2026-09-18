use super::{decode, encode};

#[test]
fn single_byte_values_round_trip() {
    for value in [0, 1, 2, 15, 127] {
        let bytes = encode(value);
        assert_eq!(bytes.len(), 1, "{value} should fit in one byte");
        assert_eq!(decode(&bytes).unwrap(), Some((value, 1)));
    }
}

#[test]
fn multi_byte_values_round_trip() {
    for value in [128, 255, 300, 2_097_151, 2_097_152, i32::MAX] {
        let bytes = encode(value);
        assert!(bytes.len() > 1, "{value} should need more than one byte");
        assert_eq!(decode(&bytes).unwrap(), Some((value, bytes.len())));
    }
}

/// `-1`'s bit pattern is all ones: the widest VarInt the protocol produces,
/// and the one `protocol_version` sends since this crate claims no
/// particular version.
#[test]
fn negative_values_take_the_full_five_bytes() {
    let bytes = encode(-1);
    assert_eq!(bytes.len(), 5);
    assert_eq!(decode(&bytes).unwrap(), Some((-1, 5)));

    let bytes = encode(i32::MIN);
    assert_eq!(bytes.len(), 5);
    assert_eq!(decode(&bytes).unwrap(), Some((i32::MIN, 5)));
}

/// A byte carrying the continuation bit with nothing after it isn't
/// invalid — it just isn't complete yet. The caller reads more off the
/// socket and tries again.
#[test]
fn a_truncated_buffer_asks_for_more_rather_than_failing() {
    let bytes = encode(300);
    assert_eq!(decode(&bytes[..1]).unwrap(), None);
}

#[test]
fn an_empty_buffer_asks_for_more() {
    assert_eq!(decode(&[]).unwrap(), None);
}

/// Every byte says "more to come": a VarInt this long doesn't exist in the
/// protocol. Looping until the sender stops would let a hostile server hold
/// the connection open forever; refusing at the 5th is what closes that
/// door.
#[test]
fn five_bytes_that_never_terminate_are_refused() {
    let hostile = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    assert!(decode(&hostile).is_err());
}

/// The same refusal holds even when a 6th byte — a valid terminator — is
/// sitting right there: [`decode`] must not read it to find that out.
#[test]
fn a_sixth_byte_is_never_consulted() {
    let hostile = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    assert!(decode(&hostile).is_err());
}
