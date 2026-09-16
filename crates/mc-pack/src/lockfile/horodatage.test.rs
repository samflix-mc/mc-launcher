use super::format_iso8601;

#[test]
fn horodatage_iso8601() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    // Année bissextile, 29 février.
    assert_eq!(format_iso8601(1_709_164_800), "2024-02-29T00:00:00Z");
}
