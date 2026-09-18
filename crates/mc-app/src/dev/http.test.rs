use super::{Response, announced_length, parse, render};

const REQUEST: &str = "POST /command/status HTTP/1.1\r\n\
Host: 127.0.0.1:1421\r\n\
Content-Type: application/json\r\n\
Content-Length: 2\r\n\
\r\n\
{}";

#[test]
fn an_ordinary_request_parses() {
    let (method, path, headers) = parse(REQUEST).expect("readable request");

    assert_eq!(method, "POST");
    assert_eq!(path, "/command/status");
    assert_eq!(headers.get("content-length").map(String::as_str), Some("2"));
    assert_eq!(announced_length(&headers), 2);
}

/// Header names aren't case-sensitive, and clients don't hold back. Comparing
/// them as-is would miss `Content-Length` written differently — and the body
/// would then be truncated to zero bytes.
#[test]
fn headers_parse_regardless_of_case() {
    let raw = "POST /x HTTP/1.1\r\nCONTENT-LENGTH: 17\r\n\r\n";
    let (_, _, headers) = parse(raw).expect("readable request");

    assert_eq!(announced_length(&headers), 17);
}

/// The query string isn't part of the routing: `/command/status?t=1`
/// designates the same command, and keeping it would produce an unknown
/// command.
#[test]
fn the_query_string_does_not_count_in_the_path() {
    let (_, path, _) = parse("GET /events?since=3 HTTP/1.1\r\n\r\n").expect("readable");

    assert_eq!(path, "/events");
}

/// Without a length header, we expect NO body bytes at all.
///
/// The opposite — waiting for "whatever comes" — would hang the connection
/// until the client gives up, on any bodyless request.
#[test]
fn without_an_announced_length_nothing_is_expected() {
    let (_, _, headers) = parse("GET / HTTP/1.1\r\nHost: x\r\n\r\n").expect("readable");

    assert_eq!(announced_length(&headers), 0);
}

/// A length that isn't a number is worth zero, and doesn't panic: the
/// request comes from a client we don't control.
#[test]
fn an_unreadable_length_is_worth_zero() {
    let (_, _, headers) =
        parse("POST /x HTTP/1.1\r\nContent-Length: lots\r\n\r\n").expect("readable");

    assert_eq!(announced_length(&headers), 0);
}

#[test]
fn an_empty_request_does_not_parse() {
    assert!(parse("").is_none());
    assert!(parse("GET\r\n\r\n").is_none());
}

/// **The cross-origin headers are on EVERY response.**
///
/// The front runs on Angular's development server, so on another port:
/// without them, the browser refuses the response without anything showing
/// up server-side — you see a successful request and a front that receives
/// nothing.
#[test]
fn every_response_carries_the_cross_origin_headers() {
    let text = render(&Response::json("{}".to_string()));

    assert!(text.contains("Access-Control-Allow-Origin: *"), "{text}");
    assert!(text.contains("Access-Control-Allow-Headers: Content-Type"));
}

/// The announced length is that of the body, in BYTES.
///
/// A player's name can carry non-ASCII characters: "é" weighs two bytes for
/// one character, and announcing the character count would truncate the
/// response by one byte per accent — which shows up as invalid JSON, at
/// random.
#[test]
fn the_length_is_counted_in_bytes() {
    let body = "\"José\"".to_string();
    let expected = body.len();
    let text = render(&Response::json(body));

    assert!(
        text.contains(&format!("Content-Length: {expected}")),
        "{text}"
    );
}

/// An error serializes as a JSON STRING, exactly like `invoke` rejects. The
/// front then handles it via the same path in both transports.
#[test]
fn an_error_has_the_shape_of_the_bridge_one() {
    let response = Response::error(404, "unknown command: thing");

    assert_eq!(response.code, 404);
    assert_eq!(response.body, "\"unknown command: thing\"");
    assert_eq!(response.mime_type, "application/json");
}

#[test]
fn codes_carry_their_reason() {
    assert!(render(&Response::error(404, "x")).starts_with("HTTP/1.1 404 Not Found"));
    assert!(render(&Response::json("1".into())).starts_with("HTTP/1.1 200 OK"));
}
