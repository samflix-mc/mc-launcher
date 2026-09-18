use super::{decode_status_payload, handshake, status_request};
use crate::varint;

/// Decodes the handshake with the SAME VarInt this module builds it with —
/// which means a drift introduced in either direction, encoding or
/// decoding, would show up here rather than only at a real server.
#[test]
fn the_handshake_carries_protocol_host_port_and_next_state() {
    let packet = handshake(-1, "mc.example.com", 25566, 1);

    let (len, len_len) = varint::decode(&packet).unwrap().unwrap();
    let body = &packet[len_len..];
    assert_eq!(body.len(), len as usize, "the announced length must match");

    let (id, mut offset) = varint::decode(body).unwrap().unwrap();
    assert_eq!(id, 0);

    let (protocol_version, consumed) = varint::decode(&body[offset..]).unwrap().unwrap();
    offset += consumed;
    assert_eq!(protocol_version, -1);

    let (host_len, consumed) = varint::decode(&body[offset..]).unwrap().unwrap();
    offset += consumed;
    let host_len = host_len as usize;
    let host = std::str::from_utf8(&body[offset..offset + host_len]).unwrap();
    offset += host_len;
    assert_eq!(host, "mc.example.com");

    let port = u16::from_be_bytes([body[offset], body[offset + 1]]);
    offset += 2;
    assert_eq!(port, 25566);

    let (next_state, _) = varint::decode(&body[offset..]).unwrap().unwrap();
    assert_eq!(next_state, 1);
}

#[test]
fn the_status_request_carries_only_its_id() {
    // Length 1 (the payload is one byte), then that byte: id 0.
    assert_eq!(status_request(), vec![1, 0]);
}

#[test]
fn a_status_response_s_json_is_extracted() {
    let json = r#"{"players":{"online":1,"max":20}}"#;
    let mut payload = varint::encode(0);
    payload.extend_from_slice(&varint::encode(json.len() as i32));
    payload.extend_from_slice(json.as_bytes());

    assert_eq!(decode_status_payload(&payload).unwrap(), json);
}

#[test]
fn a_response_with_the_wrong_id_is_rejected() {
    let mut payload = varint::encode(1); // not a status response
    payload.extend_from_slice(&varint::encode(2));
    payload.extend_from_slice(b"{}");

    assert!(decode_status_payload(&payload).is_err());
}

#[test]
fn a_payload_missing_its_string_length_is_rejected() {
    let payload = varint::encode(0); // id only, nothing after it
    assert!(decode_status_payload(&payload).is_err());
}

#[test]
fn a_string_shorter_than_announced_is_rejected() {
    let mut payload = varint::encode(0);
    payload.extend_from_slice(&varint::encode(10)); // announces 10 bytes
    payload.extend_from_slice(b"short"); // only 5 are there

    assert!(decode_status_payload(&payload).is_err());
}
