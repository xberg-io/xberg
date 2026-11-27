//! RFC 3339 timestamps for benchmark artifacts.

const RFC3339_UTC: &str = "%Y-%m-%dT%H:%M:%S%.f+00:00";
const RFC3339_OFFSET: &str = "%Y-%m-%dT%H:%M:%S%.f%:z";

/// Current UTC time as RFC 3339 with a `+00:00` offset.
pub fn now_utc_rfc3339() -> String {
    format_utc_rfc3339(jiff::Timestamp::now())
}

/// Current local time as RFC 3339 with the system UTC offset.
pub fn now_local_rfc3339() -> String {
    jiff::Zoned::now().strftime(RFC3339_OFFSET).to_string()
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
    fn now_utc_is_parseable() {
        let now = now_utc_rfc3339();
        assert!(now.ends_with("+00:00"), "{now}");
        now.parse::<jiff::Timestamp>().expect("RFC 3339 timestamp");
    }

    #[test]
    fn now_local_is_parseable_with_numeric_offset() {
        let now = now_local_rfc3339();
        now.parse::<jiff::Timestamp>().expect("RFC 3339 timestamp");
        let offset = &now[now.len() - 6..];
        assert!(offset.starts_with('+') || offset.starts_with('-'), "{now}");
        assert_eq!(&offset[3..4], ":", "{now}");
    }
}
