//! RFC 3339 timestamps for benchmark artifacts.

const RFC3339_UTC: &str = "%Y-%m-%dT%H:%M:%S%.f+00:00";
const RFC3339_OFFSET: &str = "%Y-%m-%dT%H:%M:%S%.f%:z";

/// Current UTC time as RFC 3339 with a `+00:00` offset.
pub fn now_utc_rfc3339() -> String {
    format_utc_rfc3339(jiff::Timestamp::now())
}

/// Current local time as RFC 3339 with the system UTC offset.
pub fn now_local_rfc3339() -> String {
    jiff::ZonedDateTime::now().strftime(RFC3339_OFFSET).to_string()
}

/// Strict RFC 3339 check: `T` separator and a `Z` or `+HH:MM` / `-HH:MM` offset.
///
/// `jiff::Timestamp` parsing alone also accepts a space separator and an offset
/// without minutes, which RFC 3339 rejects.
pub fn is_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    let separator_ok = matches!(bytes.get(10), Some(b'T' | b't'));
    let offset_ok = match bytes.last() {
        Some(b'Z' | b'z') => true,
        _ => {
            bytes.len() >= 6
                && matches!(bytes[bytes.len() - 6], b'+' | b'-')
                && bytes[bytes.len() - 3] == b':'
        }
    };
    separator_ok && offset_ok && value.parse::<jiff::Timestamp>().is_ok()
}

fn format_utc_rfc3339(ts: jiff::Timestamp) -> String {
    ts.strftime(RFC3339_UTC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_format_has_offset_and_optional_fraction() {
        let whole = jiff::Timestamp::new(1_700_000_000, 0).unwrap();
        assert_eq!(format_utc_rfc3339(whole), "2023-11-14T22:13:20+00:00");
        let frac = jiff::Timestamp::new(1_700_000_000, 123_456_789).unwrap();
        assert_eq!(format_utc_rfc3339(frac), "2023-11-14T22:13:20.123456789+00:00");
    }

    #[test]
    fn is_rfc3339_rejects_what_jiff_alone_accepts() {
        assert!(is_rfc3339("2023-07-15T16:30:00Z"));
        assert!(is_rfc3339("2023-07-15T16:30:00.123456789+00:00"));
        assert!(is_rfc3339("2023-07-15T16:30:00-04:00"));
        assert!(!is_rfc3339("2023-07-15 16:30:00-04"));
        assert!(!is_rfc3339("2023-07-15T16:30:00-04"));
        assert!(!is_rfc3339("2023-07-15T16:30:00"));
        assert!(!is_rfc3339("not a timestamp"));
        assert!(!is_rfc3339(""));
    }

    #[test]
    fn now_utc_is_parseable() {
        let now = now_utc_rfc3339();
        assert!(now.ends_with("+00:00"), "{now}");
        assert!(is_rfc3339(&now), "{now}");
    }

    #[test]
    fn now_local_is_parseable_with_numeric_offset() {
        let now = now_local_rfc3339();
        assert!(is_rfc3339(&now), "{now}");
        let offset = &now[now.len() - 6..];
        assert!(offset.starts_with('+') || offset.starts_with('-'), "{now}");
        assert_eq!(&offset[3..4], ":", "{now}");
    }
}
