//! IPv4 and IPv6 address detection.
//!
//! IPv4 uses dotted-quad with each octet in 0-255.
//! IPv6 supports the full and double-colon (`::`) shortened forms.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_IPV4: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}\b")
        .expect("ipv4 regex compiles")
});

static RE_IPV6: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?xi)
        (?:
            [0-9A-F]{1,4}(?::[0-9A-F]{1,4}){7}                 # full 8 groups
            |
            (?:[0-9A-F]{1,4}:){1,7}:                            # leading :: shortcut
            |
            :(?::[0-9A-F]{1,4}){1,7}                            # trailing :: shortcut
            |
            (?:[0-9A-F]{1,4}:){1,6}:[0-9A-F]{1,4}               # middle :: shortcut
        )
        ",
    )
    .expect("ipv6 regex compiles")
});

/// Find all IPv4 and IPv6 address spans in `text`.
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::IpAddress)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for m in RE_IPV4.find_iter(text) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        eligible(matched)?;
    }
    for m in RE_IPV6.find_iter(text) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        let raw = m.as_str();
        if raw.matches(':').count() < 2 {
            continue;
        }
        eligible(matched)?;
    }
    Ok(())
}
