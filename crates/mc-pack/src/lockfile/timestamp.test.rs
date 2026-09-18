use super::format_iso8601;

#[test]
fn iso8601_timestamp() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    // Leap year, February 29.
    assert_eq!(format_iso8601(1_709_164_800), "2024-02-29T00:00:00Z");
}

/// The calendar computation counts in four-hundred-year “eras” and corrects
/// non-leap centuries with two divisions that don't show before 2100: until
/// then, the term is zero and either sign gives the same date. These are
/// the only bounds that put them into play.
#[test]
fn non_leap_centuries_are_counted() {
    // 1900, 2100, 2200 and 2300 aren't leap years, 2000 and 2400 are.
    assert_eq!(format_iso8601(4_107_456_000), "2100-02-28T00:00:00Z");
    assert_eq!(format_iso8601(4_107_542_400), "2100-03-01T00:00:00Z");
    // The start of the current era, and its very last day four hundred
    // years later: the two ends of the cycle.
    assert_eq!(format_iso8601(951_868_800), "2000-03-01T00:00:00Z");
    assert_eq!(format_iso8601(13_574_563_200), "2400-02-29T00:00:00Z");
}
